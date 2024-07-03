use crate::structs::PacketFinalClassificationData as BFinalClassificationData;

use super::ToProtoMessage;

include!(concat!(env!("OUT_DIR"), "/protos.final_classification.rs"));

impl ToProtoMessage for BFinalClassificationData {
    type ProtoType = PacketFinalClassificationData;

    fn to_proto(&self) -> Option<Self::ProtoType> {
        let mut classification_data = Vec::with_capacity(22);

        for data in &self.classification_data {
            let mut tyre_stints_actual = Vec::with_capacity(8);
            let mut tyre_stints_visual = Vec::with_capacity(8);
            let mut tyre_stints_end_laps = Vec::with_capacity(8);

            for x in data.tyre_stints_actual {
                tyre_stints_actual.push(x as u32);
            }

            for x in data.tyre_stints_visual {
                tyre_stints_visual.push(x as u32);
            }

            for x in data.tyre_stints_end_laps {
                tyre_stints_end_laps.push(x as u32);
            }

            classification_data.push(FinalClassificationData {
                position: data.position as u32,
                num_laps: data.num_laps as u32,
                grid_position: data.grid_position as u32,
                points: data.points as u32,
                num_pit_stops: data.num_pit_stops as u32,
                result_status: data.result_status as u32,
                best_lap_time_in_ms: data.best_lap_time_in_ms,
                total_race_time: data.total_race_time,
                penalties_time: data.penalties_time as u32,
                num_penalties: data.num_penalties as u32,
                num_tyre_stints: data.num_tyre_stints as u32,
                tyre_stints_actual,
                tyre_stints_visual,
                tyre_stints_end_laps,
            })
        }

        Some(PacketFinalClassificationData {
            num_cars: self.num_cars as u32,
            classification_data,
        })
    }
}
