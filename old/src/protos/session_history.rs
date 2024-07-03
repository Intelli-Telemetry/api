use crate::structs::PacketSessionHistoryData as BPacketSessionHistoryData;

use super::ToProtoMessage;

include!(concat!(env!("OUT_DIR"), "/protos.session_history.rs"));

impl ToProtoMessage for BPacketSessionHistoryData {
    type ProtoType = PacketSessionHistoryData;

    fn to_proto(&self) -> Option<Self::ProtoType> {
        let mut lap_history_data = Vec::with_capacity(100);
        let mut tyre_stints_history_data = Vec::with_capacity(8);

        for lap in &self.lap_history_data {
            if lap.sector1_time_in_ms == 0 {
                break;
            }

            lap_history_data.push(LapHistoryData {
                lap_time_in_ms: lap.lap_time_in_ms,
                sector1_time: lap.sector1_time_in_ms as u32,
                sector2_time: lap.sector2_time_in_ms as u32,
                sector3_time: lap.sector3_time_in_ms as u32,
                lap_valid_bit_flags: lap.lap_valid_bit_flags as u32,
            });
        }

        for stint in &self.tyre_stints_history_data {
            if stint.tyre_actual_compound == 0 {
                break;
            }

            tyre_stints_history_data.push(TyreStintHistoryData {
                end_lap: stint.end_lap as u32,
                tyre_actual_compound: stint.tyre_actual_compound as u32,
                tyre_visual_compound: stint.tyre_visual_compound as u32,
            });
        }

        Some(PacketSessionHistoryData {
            car_idx: self.car_idx as u32,
            num_laps: self.num_laps as u32,
            num_tyre_stints: self.num_tyre_stints as u32,
            best_lap_time_lap_num: self.best_lap_time_lap_num as u32,
            best_sector1_lap_num: self.best_sector1_lap_num as u32,
            best_sector2_lap_num: self.best_sector2_lap_num as u32,
            best_sector3_lap_num: self.best_sector3_lap_num as u32,
            lap_history_data,
            tyre_stints_history_data,
        })
    }
}
