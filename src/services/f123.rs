use super::FirewallService;
use crate::{
    config::Database,
    dtos::F123Data,
    error::{AppResult, SocketError},
    protos::{packet_header::PacketType, ToProtoMessage},
};
use bb8_redis::redis::AsyncCommands;
use rustc_hash::FxHashMap;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    net::UdpSocket,
    sync::{
        broadcast::{Receiver, Sender},
        RwLock,
    },
    task::JoinHandle,
    time::timeout,
};
use tracing::{error, info};

const F123_HOST: &str = "0.0.0.0";
const DATA_PERSISTENCE: usize = 15 * 60;
const F123_MAX_PACKET_SIZE: usize = 1460;
const BASE_REDIS_KEY: &str = "f123_service:championships";

// Constants durations & timeouts
const SESSION_INTERVAL: Duration = Duration::from_secs(10);
const MOTION_INTERVAL: Duration = Duration::from_millis(700);
const SOCKET_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const SESSION_HISTORY_INTERVAL: Duration = Duration::from_secs(2);

type F123Channel = Arc<Sender<Vec<u8>>>;
type Sockets = Arc<RwLock<FxHashMap<i32, Arc<JoinHandle<()>>>>>;
type Channels = Arc<RwLock<FxHashMap<i32, F123Channel>>>;

pub struct F123Service {
    db_conn: Arc<Database>,
    sockets: Sockets,
    channels: Channels,
    firewall: Arc<FirewallService>,
}

impl F123Service {
    pub fn new(db_conn: &Arc<Database>, firewall_service: Arc<FirewallService>) -> Self {
        Self {
            db_conn: db_conn.clone(),
            firewall: firewall_service,
            channels: Arc::new(RwLock::new(FxHashMap::default())),
            sockets: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    pub async fn new_socket(&self, port: i32, championship_id: Arc<i32>) -> AppResult<()> {
        {
            let sockets = self.sockets.read().await;
            let channels = self.channels.read().await;

            if sockets.contains_key(&championship_id) || channels.contains_key(&championship_id) {
                error!("Trying to create a new socket or channel for an existing championship: {championship_id:?}");
                return Err(SocketError::AlreadyExists.into());
            }
        }

        let socket = self.spawn_socket(championship_id.clone(), port).await;

        let mut sockets = self.sockets.write().await;
        sockets.insert(*championship_id, Arc::new(socket));
        Ok(())
    }

    pub async fn active_sockets(&self) -> Vec<i32> {
        let sockets = self.sockets.read().await;
        sockets.iter().map(|entry| *entry.0).collect()
    }

    pub async fn stop_socket(&self, championship_id: i32) -> AppResult<()> {
        let mut channels = self.channels.write().await;
        let mut sockets = self.sockets.write().await;

        let channel_removed = channels.remove(&championship_id).is_some();

        let socket_removed_and_aborted = if let Some(socket) = sockets.remove(&championship_id) {
            socket.abort();
            true
        } else {
            false
        };

        if !channel_removed && !socket_removed_and_aborted {
            Err(SocketError::NotFound)?;
        }

        info!("Socket stopped for championship: {}", championship_id);
        Ok(())
    }

    async fn external_close_socket(channels: &Channels, sockets: &Sockets, championship_id: &i32) {
        let mut sockets = sockets.write().await;
        let mut channels = channels.write().await;

        if let Some(socket) = sockets.remove(championship_id) {
            socket.abort();
        }

        channels.remove(championship_id);
    }

    pub async fn championship_socket(&self, id: &i32) -> bool {
        let sockets = self.sockets.read().await;
        sockets.contains_key(id)
    }

    pub async fn get_receiver(&self, championship_id: &i32) -> Option<Receiver<Vec<u8>>> {
        let channels = self.channels.read().await;
        let channel = channels.get(championship_id);

        Some(channel.unwrap().subscribe())
    }

    async fn spawn_socket(&self, championship_id: Arc<i32>, port: i32) -> JoinHandle<()> {
        let db = self.db_conn.clone();
        let firewall = self.firewall.clone();
        let sockets = self.sockets.clone();
        let channels = self.channels.clone();

        tokio::spawn(async move {
            let mut port_partial_open = false;
            let mut buf = [0u8; F123_MAX_PACKET_SIZE];
            let redis = db.redis.clone();

            let mut last_session_update = Instant::now();
            let mut last_car_motion_update = Instant::now();
            let mut last_participants_update = Instant::now();

            // Session History Data
            let mut last_car_lap_update: FxHashMap<u8, Instant> = FxHashMap::default();
            let mut car_lap_sector_data: FxHashMap<u8, (u16, u16, u16)> = FxHashMap::default();

            // Define channel
            let (tx, _rx) = tokio::sync::broadcast::channel::<Vec<u8>>(50);

            let Ok(socket) = UdpSocket::bind(format!("{F123_HOST}:{port}")).await else {
                error!("There was an error binding to the socket for championship: {championship_id:?}");
                return;
            };

            info!("Listening for F123 data on port: {port} for championship: {championship_id:?}");

            {
                let mut channels = channels.write().await;
                channels.insert(*championship_id, Arc::new(tx.clone()));
            }

            firewall.open(*championship_id, port).await.unwrap();

            loop {
                match timeout(SOCKET_TIMEOUT, socket.recv_from(&mut buf)).await {
                    Ok(Ok((size, address))) => {
                        let buf = &buf[..size];

                        if !port_partial_open {
                            firewall
                                .open_partially(*championship_id, address.ip())
                                .await
                                .unwrap();

                            port_partial_open = true;
                        }

                        let Some(header) = F123Data::deserialize_header(buf) else {
                            error!("Error deserializing F123 header, for championship: {championship_id:?}");
                            continue;
                        };

                        if header.packet_format != 2023 {
                            error!("Not supported client");
                            break;
                        };

                        let session_id = header.session_uid;
                        if session_id.eq(&0) {
                            continue;
                        }

                        let Some(packet) = F123Data::deserialize(header.packet_id.into(), buf)
                        else {
                            continue;
                        };

                        let now = Instant::now();

                        match packet {
                            F123Data::Motion(motion_data) => {
                                if now
                                    .duration_since(last_car_motion_update)
                                    .ge(&MOTION_INTERVAL)
                                {
                                    let packet = motion_data
                                        .convert_and_encode(PacketType::CarMotion)
                                        .expect("Error converting motion data to proto message");

                                    let mut redis =
                                        redis.get().await.expect("Error getting redis connection");

                                    if let Err(e) = redis
                                        .set_ex::<&str, &[u8], ()>(
                                            &format!(
                                                "{BASE_REDIS_KEY}:{championship_id}:motion_data"
                                            ),
                                            &packet,
                                            DATA_PERSISTENCE,
                                        )
                                        .await
                                    {
                                        error!("Error saving motion to redis: {}", e);
                                    };

                                    tx.send(packet).unwrap();
                                    last_car_motion_update = now;
                                }
                            }

                            F123Data::Session(session_data) => {
                                if now
                                    .duration_since(last_session_update)
                                    .ge(&SESSION_INTERVAL)
                                {
                                    let packet = session_data
                                        .convert_and_encode(PacketType::SessionData)
                                        .expect("Error converting session data to proto message");

                                    let mut redis =
                                        redis.get().await.expect("Error getting redis connection");

                                    if let Err(e) = redis
                                        .set_ex::<&str, &[u8], ()>(
                                            &format!(
                                                "{BASE_REDIS_KEY}:{championship_id}:session_data"
                                            ),
                                            &packet,
                                            DATA_PERSISTENCE,
                                        )
                                        .await
                                    {
                                        error!("Error saving session to redis: {}", e);
                                    };

                                    tx.send(packet).unwrap();
                                    last_session_update = now;
                                }
                            }

                            F123Data::Participants(participants_data) => {
                                if now
                                    .duration_since(last_participants_update)
                                    .lt(&SESSION_INTERVAL)
                                {
                                    let packet = participants_data
                                        .convert_and_encode(PacketType::Participants)
                                        .expect(
                                            "Error converting participants data to proto message",
                                        );

                                    let mut redis =
                                        redis.get().await.expect("Error getting redis connection");

                                    if let Err(e) = redis
                                        .set_ex::<&str, &[u8], ()>(
                                            &format!("{BASE_REDIS_KEY}:{championship_id}:participants_data"),
                                            &packet,
                                            DATA_PERSISTENCE,
                                        )
                                        .await {
                                        error!("Error saving participants to redis: {}", e);
                                        };

                                    tx.send(packet).unwrap();
                                    last_participants_update = now;
                                }
                            }

                            // TODO: Export this to a different service
                            F123Data::Event(event_data) => {
                                let Some(packet) =
                                    event_data.convert_and_encode(PacketType::EventData)
                                else {
                                    continue;
                                };

                                let string_code =
                                    std::str::from_utf8(&event_data.event_string_code)
                                        .expect("Error converting string code");

                                let mut redis =
                                    redis.get().await.expect("Error getting redis connection");

                                if let Err(e) = redis.rpush::<&str, &[u8], ()>(&format!("{BASE_REDIS_KEY}:{championship_id}:events:{string_code}"), &packet).await {
                                    error!("Error saving event to redis: {}", e);
                                };

                                tx.send(packet).unwrap();
                            }

                            F123Data::SessionHistory(session_history) => {
                                let last_update = last_car_lap_update
                                    .entry(session_history.car_idx)
                                    .or_insert(now);

                                if now
                                    .duration_since(*last_update)
                                    .gt(&SESSION_HISTORY_INTERVAL)
                                {
                                    let lap = (session_history.num_laps as usize) - 1; // Lap is 0 indexed

                                    let sectors = (
                                        session_history.lap_history_data[lap].sector1_time_in_ms,
                                        session_history.lap_history_data[lap].sector2_time_in_ms,
                                        session_history.lap_history_data[lap].sector3_time_in_ms,
                                    );

                                    let last_sectors = car_lap_sector_data
                                        .entry(session_history.car_idx)
                                        .or_insert(sectors);

                                    if sectors == *last_sectors {
                                        continue;
                                    }

                                    let car_idx = session_history.car_idx;
                                    let packet = session_history
                                        .convert_and_encode(PacketType::SessionHistoryData)
                                        .expect("Error converting history data to proto message");

                                    let mut redis =
                                        redis.get().await.expect("Error getting redis connection");

                                    if let Err(e) =redis
                                    .set_ex::<&str, &[u8], ()>(
                                        &format!("f123_service:championships:{championship_id}:session_history:{car_idx}"),
                                        &packet,
                                        DATA_PERSISTENCE,
                                    )
                                    .await {
                                        error!("Error saving session history to redis: {}", e);
                                    };

                                    tx.send(packet).unwrap();

                                    *last_update = now;
                                    *last_sectors = sectors;
                                }
                            }

                            //TODO Collect All data from redis and save it to the mariadb database
                            F123Data::FinalClassification(classification_data) => {
                                let packet = classification_data
                                    .convert_and_encode(PacketType::FinalClassificationData)
                                    .expect("Error converting final classification data to proto message");

                                // TODO: If session type is race save all session data in the database and close the socket
                                tx.send(packet).unwrap();
                            }
                        }
                    }

                    Ok(Err(e)) => {
                        error!("Error receiving data from F123 socket: {}", e);
                        info!("Stopping socket for championship: {}", championship_id);
                        firewall.close(&championship_id).await.unwrap();
                        Self::external_close_socket(&channels, &sockets, &championship_id).await;
                        break;
                    }

                    Err(_) => {
                        info!("Socket  timeout for championship: {}", championship_id);
                        firewall.close(&championship_id).await.unwrap();
                        Self::external_close_socket(&channels, &sockets, &championship_id).await;
                    }
                }
            }
        })
    }
}
