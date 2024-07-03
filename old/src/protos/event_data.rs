use tracing::warn;

use crate::{
    protos::event_data::event_data_details::Details,
    structs::{
        EventCode, EventDataDetails as BEventDataDetails, PacketEventData as BPacketEventData,
    },
};

use super::ToProtoMessage;

include!(concat!(env!("OUT_DIR"), "/protos.event_data.rs"));

const EVENT_NOT_SEND: [EventCode; 9] = [
    EventCode::ButtonStatus,
    EventCode::TeamMateInPits,
    EventCode::Flashback,
    EventCode::SessionEnded,
    EventCode::DRSEnabled,
    EventCode::DRSDisabled,
    EventCode::ChequeredFlag,
    EventCode::RedFlag,
    EventCode::LightsOut,
];

impl ToProtoMessage for BPacketEventData {
    type ProtoType = PacketEventData;

    fn to_proto(&self) -> Option<Self::ProtoType> {
        let Ok(event_code) = EventCode::try_from(&self.event_string_code) else {
            warn!("Unknown event code: {:?}", self.event_string_code);
            return None;
        };

        if EVENT_NOT_SEND.contains(&event_code) {
            return None;
        };

        Some(PacketEventData {
            event_string_code: self.event_string_code.to_vec(),
            event_details: Some(convert_event_data_details(&event_code, &self.event_details)),
        })
    }
}

#[inline(always)]
pub fn convert_event_data_details(
    event_code: &EventCode,
    event_data_details: &BEventDataDetails,
) -> EventDataDetails {
    let details = match event_code {
        EventCode::FastestLap => {
            let fastest_lap = unsafe { &event_data_details.fastest_lap };

            Details::FastestLap(FastestLap {
                lap_time: fastest_lap.lap_time,
                vehicle_idx: fastest_lap.vehicle_idx as u32,
            })
        }

        EventCode::Retirement => {
            let retirement = unsafe { &event_data_details.retirement };

            Details::Retirement(Retirement {
                vehicle_idx: retirement.vehicle_idx as u32,
            })
        }

        EventCode::RaceWinner => {
            let race_winner = unsafe { &event_data_details.race_winner };

            Details::RaceWinner(RaceWinner {
                vehicle_idx: race_winner.vehicle_idx as u32,
            })
        }

        EventCode::PenaltyIssued => {
            let penalty = unsafe { &event_data_details.penalty };

            Details::Penalty(Penalty {
                penalty_type: penalty.penalty_type as u32,
                infringement_type: penalty.infringement_type as u32,
                vehicle_idx: penalty.vehicle_idx as u32,
                other_vehicle_idx: penalty.other_vehicle_idx as u32,
                time: penalty.time as u32,
                lap_num: penalty.lap_num as u32,
                places_gained: penalty.places_gained as u32,
            })
        }

        EventCode::SpeedTrapTriggered => {
            let speed_trap = unsafe { &event_data_details.speed_trap };

            Details::SpeedTrap(SpeedTrap {
                speed: speed_trap.speed,
                vehicle_idx: speed_trap.vehicle_idx as u32,
                is_overall_fastest_in_session: speed_trap.is_overall_fastest_in_session as u32,
                is_driver_fastest_in_session: speed_trap.is_driver_fastest_in_session as u32,
                fastest_vehicle_idx_in_session: speed_trap.fastest_vehicle_idx_in_session as u32,
                fastest_speed_in_session: speed_trap.fastest_speed_in_session,
            })
        }

        EventCode::StartLights => {
            let start_lights = unsafe { &event_data_details.start_lights };

            Details::StartLights(StartLights {
                num_lights: start_lights.num_lights as u32,
            })
        }

        EventCode::DriveThroughServed => {
            let drive_through = unsafe { &event_data_details.drive_through_penalty_served };

            Details::DriveThroughPenaltyServed(DriveThroughPenaltyServed {
                vehicle_idx: drive_through.vehicle_idx as u32,
            })
        }

        EventCode::StopGoServed => {
            let stop_go = unsafe { &event_data_details.stop_go_penalty_served };

            Details::StopGoPenaltyServed(StopGoPenaltyServed {
                vehicle_idx: stop_go.vehicle_idx as u32,
            })
        }

        EventCode::Overtake => {
            let overtake = unsafe { &event_data_details.overtake };

            Details::Overtake(Overtake {
                overtaking_vehicle_idx: overtake.overtaking_vehicle_idx as u32,
                being_overtaken_vehicle_idx: overtake.being_overtaken_vehicle_idx as u32,
            })
        }

        EventCode::SessionStarted => {
            let session_started = unsafe { &event_data_details.start_lights };

            Details::StartLights(StartLights {
                num_lights: session_started.num_lights as u32,
            })
        }

        // This should never happen because we are filtering out these events
        _ => unreachable!(),
    };

    EventDataDetails {
        details: Some(details),
    }
}
