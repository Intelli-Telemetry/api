use ahash::{AHashMap, AHashSet};
use async_compression::{tokio::write::ZstdEncoder, Level};
use ntex::util::Bytes;
use parking_lot::RwLock;
use tokio::{io::AsyncWriteExt, time::Instant};
use tracing::error;

use crate::{
    config::constants::F1_CACHING_DUR,
    error::AppResult,
    protos::{batched::ToProtoMessageBatched, packet_header::PacketType, PacketHeader},
    structs::OptionalMessage,
};

struct CachedData(Bytes, Instant);

pub struct PacketCaching {
    car_motion: Option<Vec<u8>>,
    session_data: Option<Vec<u8>>,
    participants: Option<Vec<u8>>,
    history_data: AHashMap<u8, Vec<u8>>,
    event_data: AHashSet<Vec<u8>>,
    cache: RwLock<Option<CachedData>>,
}

impl PacketCaching {
    pub fn new() -> Self {
        Self {
            car_motion: None,
            session_data: None,
            participants: None,
            history_data: AHashMap::with_capacity(20),
            event_data: AHashSet::with_capacity(10),
            cache: RwLock::new(None),
        }
    }

    pub async fn get(&self) -> AppResult<Option<Bytes>> {
        {
            let cache_read = self.cache.read();
            if let Some(cached) = &*cache_read {
                if cached.1.elapsed() < F1_CACHING_DUR {
                    return Ok(Some(cached.0.clone()));
                }
            }
        }

        let mut headers = Vec::with_capacity(self.total_headers());

        if let Some(header) = self.get_car_motion() {
            headers.push(header);
        }

        if let Some(header) = self.get_session_data() {
            headers.push(header)
        };

        if let Some(header) = self.get_participants() {
            headers.push(header)
        };

        if let Some(mut history_headers) = self.get_history_data() {
            headers.append(&mut history_headers)
        };

        if let Some(mut events_headers) = self.get_events_data() {
            headers.append(&mut events_headers)
        };

        if headers.is_empty() {
            return Ok(None);
        }

        match ToProtoMessageBatched::batched_encoded(headers) {
            None => Ok(None),
            Some(bytes) => {
                let compressed = Self::compress(&bytes).await?;
                let mut cache_write = self.cache.write();

                *cache_write = Some(CachedData(compressed.clone(), Instant::now()));
                Ok(Some(compressed))
            }
        }
    }

    pub fn save(
        &mut self,
        packet_type: PacketType,
        payload: &[u8],
        second_param: Option<OptionalMessage>,
    ) {
        match packet_type {
            PacketType::CarMotion => self.set_car_motion(payload),
            PacketType::SessionData => self.set_session_data(payload),
            PacketType::Participants => self.set_participants(payload),

            PacketType::EventData => {
                debug_assert!(second_param.is_some());

                if let Some(OptionalMessage::Code(code)) = second_param {
                    // Todo - try to avoid .to_vec()
                    self.push_event(payload.to_vec(), code);
                } else {
                    error!("Error Receiving OptionalMessage");
                }
            }

            PacketType::SessionHistoryData => {
                debug_assert!(second_param.is_some());

                if let Some(OptionalMessage::Number(car_id)) = second_param {
                    // Todo - try to avoid .to_vec()
                    self.set_history_data(payload.to_vec(), car_id)
                } else {
                    error!("Error Receiving OptionalMessage");
                }
            }

            PacketType::FinalClassificationData => {
                todo!()
            }
        }
    }

    fn total_headers(&self) -> usize {
        let base_count = 3;
        let history_estimate = self.history_data.len();
        let events_estimate = self.event_data.len();

        base_count + history_estimate + events_estimate
    }

    #[inline(always)]
    fn get_car_motion(&self) -> Option<PacketHeader> {
        self.car_motion
            .as_ref()
            .map(|car_motion_data| PacketHeader {
                r#type: PacketType::CarMotion.into(),
                payload: car_motion_data.clone(),
            })
    }

    #[inline(always)]
    fn get_participants(&self) -> Option<PacketHeader> {
        self.participants.as_ref().map(|participants| PacketHeader {
            r#type: PacketType::Participants.into(),
            payload: participants.clone(),
        })
    }

    #[inline(always)]
    fn get_session_data(&self) -> Option<PacketHeader> {
        self.session_data.as_ref().map(|session_data| PacketHeader {
            r#type: PacketType::SessionData.into(),
            payload: session_data.clone(),
        })
    }

    #[inline(always)]
    fn get_history_data(&self) -> Option<Vec<PacketHeader>> {
        let len = self.history_data.len();
        if len == 0 {
            return None;
        }

        let mut vec = Vec::with_capacity(len);
        for (_, session_history) in &self.history_data {
            vec.push(PacketHeader {
                r#type: PacketType::SessionHistoryData.into(),
                payload: session_history.clone(),
            })
        }

        Some(vec)
    }

    #[inline(always)]
    fn get_events_data(&self) -> Option<Vec<PacketHeader>> {
        let len = self.event_data.len();

        if len == 0 {
            return None;
        }

        let mut vec = Vec::with_capacity(len);
        for event in &self.event_data {
            vec.push(PacketHeader {
                r#type: PacketType::EventData.into(),
                payload: event.clone(),
            })
        }

        Some(vec)
    }

    #[inline(always)]
    fn set_car_motion(&mut self, payload: &[u8]) {
        match &mut self.car_motion {
            Some(car_motion) => {
                car_motion.clear();
                car_motion.extend_from_slice(payload)
            }

            None => {
                let mut car_motion = Vec::with_capacity(payload.len());
                car_motion.extend_from_slice(payload);
                self.car_motion = Some(car_motion);
            }
        }
    }

    #[inline(always)]
    fn set_session_data(&mut self, payload: &[u8]) {
        match &mut self.session_data {
            Some(vec) => {
                vec.clear();
                vec.extend_from_slice(payload);
            }
            None => {
                let mut vec = Vec::with_capacity(payload.len());
                vec.extend_from_slice(payload);
                self.session_data = Some(vec);
            }
        }
    }

    #[inline(always)]
    fn set_participants(&mut self, payload: &[u8]) {
        match &mut self.participants {
            Some(vec) => {
                vec.clear();
                vec.extend_from_slice(payload);
            }
            None => {
                let mut vec = Vec::with_capacity(payload.len());
                vec.extend_from_slice(payload);
                self.participants = Some(vec);
            }
        }
    }

    #[inline(always)]
    fn set_history_data(&mut self, payload: Vec<u8>, car_idx: u8) {
        self.history_data.insert(car_idx, payload);
    }

    #[inline(always)]
    fn push_event(&mut self, payload: Vec<u8>, _code: [u8; 4]) {
        self.event_data.insert(payload);
    }

    #[inline(always)]
    async fn compress(data: &[u8]) -> AppResult<Bytes> {
        let mut encoder = ZstdEncoder::with_quality(Vec::new(), Level::Default);

        encoder.write_all(data).await.unwrap();
        encoder.shutdown().await.unwrap();

        Ok(Bytes::from(encoder.into_inner()))
    }
}
