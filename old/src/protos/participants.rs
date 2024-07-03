use std::ffi::CStr;

use crate::structs::PacketParticipantsData as BPacketParticipantsData;

use super::ToProtoMessage;

include!(concat!(env!("OUT_DIR"), "/protos.participants.rs"));

impl ToProtoMessage for BPacketParticipantsData {
    type ProtoType = PacketParticipantsData;

    fn to_proto(&self) -> Option<Self::ProtoType> {
        let mut participants = Vec::with_capacity(22);

        for participant in &self.participants {
            let name = match CStr::from_bytes_until_nul(&participant.name) {
                Ok(c_str) => match c_str.to_str() {
                    Ok(valid_str) => valid_str.to_owned(),
                    Err(_) => String::new(),
                },

                Err(_) => String::new(),
            };

            participants.push(ParticipantData {
                ai_controlled: participant.ai_controlled as u32,
                driver_id: participant.driver_id as u32,
                network_id: participant.network_id as u32,
                team_id: participant.team_id as u32,
                my_team: participant.my_team as u32,
                race_number: participant.race_number as u32,
                nationality: participant.nationality as u32,
                name,
                your_telemetry: participant.your_telemetry as u32,
                show_online_names: participant.show_online_names as u32,
                platform: participant.platform as u32,
            });
        }

        Some(PacketParticipantsData {
            num_active_cars: self.num_active_cars as u32,
            participants,
        })
    }
}
