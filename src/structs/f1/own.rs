use std::mem;
use std::sync::Arc;

use ntex::util::Bytes;
use parking_lot::RwLock;
use tokio::sync::{broadcast::Receiver, oneshot};

use crate::{
    error::{AppResult, F1ServiceError},
    services::PacketCaching,
};

use super::game::*;

pub struct F1ServiceData {
    pub cache: Arc<RwLock<PacketCaching>>,
    pub channel: Arc<Receiver<Bytes>>,
    pub shutdown_tx: oneshot::Sender<()>,
}

pub enum OptionalMessage {
    Code([u8; 4]),
    Number(u8),
}

pub enum F1Data<'a> {
    Motion(&'a PacketMotionData),
    Session(&'a PacketSessionData),
    Event(&'a PacketEventData),
    Participants(&'a PacketParticipantsData),
    FinalClassification(&'a PacketFinalClassificationData),
    SessionHistory(&'a PacketSessionHistoryData),
    CarDamage(&'a PacketCarDamageData),
    CarStatus(&'a PacketCarStatusData),
    CarTelemetry(&'a PacketCarTelemetryData),
}

impl<'a> F1Data<'a> {
    pub fn try_cast(data: &[u8]) -> AppResult<(PacketIds, &PacketHeader, F1Data)> {
        let header = Self::unsafe_cast::<PacketHeader>(data)?;

        let packet_id = PacketIds::try_from(header.packet_id).unwrap();
        let packet = match packet_id {
            PacketIds::Motion => Self::unsafe_cast::<PacketMotionData>(data).map(F1Data::Motion),

            PacketIds::Session => Self::unsafe_cast::<PacketSessionData>(data).map(F1Data::Session),

            PacketIds::Participants => {
                Self::unsafe_cast::<PacketParticipantsData>(data).map(F1Data::Participants)
            }

            PacketIds::FinalClassification => {
                Self::unsafe_cast::<PacketFinalClassificationData>(data)
                    .map(F1Data::FinalClassification)
            }

            PacketIds::SessionHistory => {
                Self::unsafe_cast::<PacketSessionHistoryData>(data).map(F1Data::SessionHistory)
            }

            PacketIds::Event => Self::unsafe_cast::<PacketEventData>(data).map(F1Data::Event),

            PacketIds::CarDamage => {
                Self::unsafe_cast::<PacketCarDamageData>(data).map(F1Data::CarDamage)
            }

            PacketIds::CarStatus => {
                Self::unsafe_cast::<PacketCarStatusData>(data).map(F1Data::CarStatus)
            }

            PacketIds::CarTelemetry => {
                Self::unsafe_cast::<PacketCarTelemetryData>(data).map(F1Data::CarTelemetry)
            }

            _ => Err(F1ServiceError::InvalidPacketType)?,
        }?;

        Ok((packet_id, header, packet))
    }

    #[inline(always)]
    fn unsafe_cast<T>(bytes: &[u8]) -> AppResult<&T> {
        if bytes.len() < mem::size_of::<T>() {
            return Err(F1ServiceError::CastingError)?;
        }

        let alignment = mem::align_of::<T>();
        if (bytes.as_ptr() as usize) % alignment != 0 {
            return Err(F1ServiceError::CastingError)?;
        }

        Ok(unsafe { &*(bytes.as_ptr() as *const T) })
    }
}
