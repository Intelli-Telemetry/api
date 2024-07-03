use crate::{
    config::constants::*,
    error::{AppResult, F1ServiceError},
    protos::{packet_header::PacketType, ToProtoMessage},
    services::f1::{batching::PacketBatching, caching::PacketCaching},
    structs::{F1Data, F1ServiceData, OptionalMessage, PacketIds, SectorsLaps, SessionType},
};
use ahash::AHashMap;
use dashmap::DashMap;
use ntex::util::Bytes;
use parking_lot::RwLock;
use std::{sync::Arc, time::Instant};
use tokio::{
    net::UdpSocket,
    sync::{
        broadcast::{channel, Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
    time::timeout,
};
use tracing::{error, info, info_span, warn};

use super::firewall::FirewallService;

type Services = DashMap<i32, F1ServiceData>;

#[derive(Clone)]
pub struct F1Service {
    services: &'static Services,
    firewall: &'static FirewallService,
}

impl F1Service {
    pub fn new() -> Self {
        let firewall = Box::leak(Box::new(FirewallService::new()));
        let services = Box::leak(Box::new(DashMap::with_capacity(100)));

        Self { firewall, services }
    }

    pub async fn active_services(&self) -> Vec<i32> {
        let len = self.services.len();
        let mut services = Vec::with_capacity(len);

        for item in self.services {
            services.push(*item.key())
        }

        services
    }

    pub async fn service_active(&self, id: i32) -> bool {
        self.services.contains_key(&id)
    }

    // Todo - Finish this implementation
    pub async fn service_cache(&self, championship_id: i32) -> AppResult<Option<Bytes>> {
        let service = self.services.get(&championship_id).unwrap();
        let cache = service.cache.read_arc();
        cache.get().await
    }

    pub async fn subscribe(&self, championship_id: i32) -> Option<Receiver<Bytes>> {
        let service = self.services.get(&championship_id)?;
        Some(service.channel.resubscribe())
    }

    pub async fn start_service(&self, port: i32, championship_id: i32) -> AppResult<()> {
        if self.services.contains_key(&championship_id) {
            return Err(F1ServiceError::AlreadyExists)?;
        }

        let (tx, rx) = channel::<Bytes>(50);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let cache = Arc::from(RwLock::from(PacketCaching::new()));

        self.create_service_thread(port, championship_id, tx, shutdown_rx, cache.clone())
            .await;

        self.services.insert(
            championship_id,
            F1ServiceData {
                shutdown_tx,
                cache,
                channel: Arc::from(rx),
            },
        );

        Ok(())
    }

    pub async fn stop_service(&self, championship_id: i32) -> AppResult<()> {
        if !self.services.contains_key(&championship_id) {
            return Err(F1ServiceError::NotActive)?;
        }

        if let Some((_, service)) = self.services.remove(&championship_id) {
            if service.shutdown_tx.send(()).is_err() {
                return Err(F1ServiceError::Shutdown)?;
            };
        } else {
            warn!("Trying to remove a not existing service");
        }

        info!("Service stopped for championship: {}", championship_id);
        Ok(())
    }

    #[inline(always)]
    async fn internal_close(services: &Services, championship_id: i32, firewall: &FirewallService) {
        if firewall.close(championship_id).await.is_err() {
            error!("Error closing port in firewall");
        }
        services.remove(&championship_id);
    }

    async fn create_service_thread(
        &self,
        port: i32,
        championship_id: i32,
        tx: Sender<Bytes>,
        mut shutdown_rx: oneshot::Receiver<()>,
        cache: Arc<RwLock<PacketCaching>>,
    ) -> JoinHandle<()> {
        let firewall = self.firewall;
        let services = self.services;

        // Todo - Implement all this logic in a structure
        tokio::spawn(async move {
            let span = info_span!("F1 Service", championship_id = championship_id);
            let _guard = span.enter();

            let mut port_partial_open = false;
            let mut buf = [0u8; BUFFER_SIZE];
            let mut session_type = None;
            let mut last_session_update = Instant::now();
            let mut last_car_motion_update = last_session_update;
            let mut last_participants_update = last_session_update;
            let close_service = Self::internal_close(services, championship_id, firewall);

            // Session History Data
            let mut last_car_lap_update: AHashMap<u8, Instant> = AHashMap::with_capacity(20);
            let mut car_lap_sector_data: AHashMap<u8, SectorsLaps> = AHashMap::with_capacity(20);

            let mut packet_batching = PacketBatching::new(tx.clone(), cache);

            let Ok(socket) = UdpSocket::bind(format!("{SOCKET_HOST}:{port}")).await else {
                error!("There was an error binding to the socket");
                close_service.await;
                return;
            };

            if firewall.open(championship_id, port as u16).await.is_err() {
                error!("Error opening port in firewall");
                close_service.await;
                return;
            }

            info!("Listening for F1 data on port: {port}");

            loop {
                tokio::select! {
                    _ = (&mut shutdown_rx) => {
                        info!("Shutting down service");
                        close_service.await;
                        return;
                    }

                    // Todo - Verify the necessity of rejecting packets from addresses other than the initial sender's, currently enforced by the firewall. Consider adding this as an additional security layer in the code.
                    result = timeout(SOCKET_TIMEOUT, socket.recv_from(&mut buf)) => {
                        match result {
                            Ok(Ok((size, address))) => {
                                let buf = &buf[..size];
                                let now = Instant::now();

                                if !port_partial_open {
                                    if firewall
                                        .restrict_to_ip(championship_id, address.ip().to_string())
                                        .await.is_err()
                                    {
                                        error!("Error restricting port to ip");
                                        close_service.await;
                                        return;
                                    }

                                    port_partial_open = true;
                                }

                                let Ok((packet_id, header, packet)) = F1Data::try_cast(buf) else {
                                    continue;
                                };

                                if header.packet_format != 2023 {
                                    close_service.await;
                                    error!("Unsupported packet format");
                                    return;
                                };

                                let session_id = header.session_uid;
                                if session_id == 0 {
                                    continue;
                                }

                                match packet_id {
                                    PacketIds::Motion => {
                                        if now.duration_since(last_car_motion_update) < MOTION_INTERVAL {
                                            continue;
                                        }
                                    }

                                    PacketIds::Session => {
                                        if now.duration_since(last_session_update) < SESSION_INTERVAL {
                                            continue;
                                        }
                                    }

                                    PacketIds::Participants => {
                                        if now.duration_since(last_participants_update) < SESSION_INTERVAL {
                                            continue;
                                        }
                                    }

                                    _ => {}
                                }

                                match packet {
                                    F1Data::Motion(motion_data) => {
                                        // Safe to unwrap cause it cannot return none
                                        let packet = motion_data.convert(PacketType::CarMotion).unwrap();

                                        last_car_motion_update = now;
                                        packet_batching.push(packet);
                                    }

                                    F1Data::Session(session_data) => {
                                        // #[cfg(not(debug_assertions))]
                                        // if session_data.network_game != 1 {
                                        //     error!("Not Online Game, closing service");
                                        //     close_service.await;
                                        //     return;
                                        // }

                                        let Ok(converted_session_type) =
                                            SessionType::try_from(session_data.session_type)
                                        else {
                                            error!("Error deserializing F1 session type");
                                            continue;
                                        };

                                        session_type = Some(converted_session_type);
                                        let packet = session_data.convert(PacketType::SessionData).unwrap();
                                        last_session_update = now;
                                        packet_batching.push(packet);
                                    }

                                    F1Data::Participants(participants_data) => {
                                        let packet =
                                            participants_data.convert(PacketType::Participants).unwrap();

                                        last_participants_update = now;
                                        packet_batching.push(packet);
                                    }

                                    // If the session is not race don't save events
                                    F1Data::Event(event_data) => {
                                        let Some(session_type) = &session_type else {
                                            continue;
                                        };

                                        if ![SessionType::R, SessionType::R2, SessionType::R3]
                                            .contains(session_type)
                                        {
                                            continue;
                                        }

                                        let Some(packet) = event_data.convert(PacketType::EventData) else {
                                            continue;
                                        };

                                        packet_batching.push_with_optional_parameter(
                                            packet,
                                            Some(OptionalMessage::Code(event_data.event_string_code)),
                                        );
                                    }

                                    F1Data::SessionHistory(session_history) => {
                                        let last_update = last_car_lap_update
                                            .entry(session_history.car_idx)
                                            .or_insert(now);

                                        if now.duration_since(*last_update) > HISTORY_INTERVAL {
                                            // Todo: Check if this comparisons are slower than just saving the data in local memory
                                            let lap = (session_history.num_laps as usize) - 1; // Lap is 0 indexed

                                            let sectors = SectorsLaps {
                                                sector1: session_history.lap_history_data[lap]
                                                    .sector1_time_in_ms,
                                                sector2: session_history.lap_history_data[lap]
                                                    .sector2_time_in_ms,
                                                sector3: session_history.lap_history_data[lap]
                                                    .sector3_time_in_ms,
                                            };

                                            let last_sectors = car_lap_sector_data
                                                .entry(session_history.car_idx)
                                                .or_insert(sectors);

                                            if sectors == *last_sectors {
                                                *last_update = now;
                                                continue;
                                            }

                                            let packet = session_history
                                                .convert(PacketType::SessionHistoryData)
                                                .unwrap();

                                            *last_update = now;
                                            *last_sectors = sectors;
                                            packet_batching.push_with_optional_parameter(
                                                packet,
                                                Some(OptionalMessage::Number(session_history.car_idx)),
                                            );
                                        }
                                    }

                                    F1Data::CarDamage(_car_damage_data) => {
                                        // info!("Car Damage: {:?}", car_damage_data);
                                    }

                                    F1Data::CarTelemetry(_car_telemetry) => {
                                        // info!("Car Telemetry: {:?}", car_telemetry);
                                    }

                                    F1Data::CarStatus(_car_status) => {
                                        // info!("Car Status: {:?}", car_status);
                                    }

                                        //TODO - Collect All data from redis and save it to the mariadb db
                                    F1Data::FinalClassification(classification_data) => {
                                        info!("FinalClassification data received");

                                        let packet = classification_data
                                            .convert(PacketType::FinalClassificationData)
                                            .unwrap();

                                        // Only for testing purposes, in the future this should close the service when the race is finished
                                        {
                                            info!("Session type: {:?}", session_type);

                                            if let SessionType::R | SessionType::R2 | SessionType::R3 =
                                                session_type.as_ref().unwrap()
                                            {
                                                info!("Race Finished, saving final classification data");
                                            }
                                        }

                                        // If session type is race save all session data in the db and close the service
                                        // Todo - this should be called after saving all data in the db
                                        packet_batching.push(packet);
                                    }
                                }
                            }

                            Ok(Err(e)) => {
                                error!("Error receiving data from udp socket: {}", e);
                                info!("Stopping socket");
                                close_service.await;
                                break;
                            }

                            Err(_) => {
                                info!("Service timeout");
                                close_service.await;
                                break;
                            }
                        }
                    }
                }
            }
        })
    }
}
