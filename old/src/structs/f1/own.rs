use std::sync::Arc;

use ntex::util::Bytes;
use parking_lot::RwLock;
use tokio::sync::{broadcast::Receiver, oneshot};

use crate::{
    error::{AppResult, F1ServiceError},
    services::PacketCaching,
    utils::cast,
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
    // Todo: Change this function name to one more idiomatic
    pub fn try_cast(data: &[u8]) -> AppResult<(PacketIds, &PacketHeader, F1Data)> {
        let header = cast::<PacketHeader>(data)?;
        let packet_id = PacketIds::try_from(header.packet_id).unwrap();

        let packet = match packet_id {
            PacketIds::Event => cast::<PacketEventData>(data).map(F1Data::Event),
            PacketIds::Motion => cast::<PacketMotionData>(data).map(F1Data::Motion),
            PacketIds::Session => cast::<PacketSessionData>(data).map(F1Data::Session),
            PacketIds::CarDamage => cast::<PacketCarDamageData>(data).map(F1Data::CarDamage),
            PacketIds::CarStatus => cast::<PacketCarStatusData>(data).map(F1Data::CarStatus),

            PacketIds::Participants => {
                cast::<PacketParticipantsData>(data).map(F1Data::Participants)
            }

            PacketIds::FinalClassification => {
                cast::<PacketFinalClassificationData>(data).map(F1Data::FinalClassification)
            }

            PacketIds::SessionHistory => {
                cast::<PacketSessionHistoryData>(data).map(F1Data::SessionHistory)
            }

            PacketIds::CarTelemetry => {
                cast::<PacketCarTelemetryData>(data).map(F1Data::CarTelemetry)
            }

            _ => Err(F1ServiceError::InvalidPacketType)?,
        }?;

        Ok((packet_id, header, packet))
    }
}
