use crate::structs::PacketMotionData as BPacketMotionData;

use super::ToProtoMessage;

include!(concat!(env!("OUT_DIR"), "/protos.car_motion_data.rs"));

impl ToProtoMessage for BPacketMotionData {
    type ProtoType = PacketMotionData;

    fn to_proto(&self) -> Option<Self::ProtoType> {
        let mut car_motion_data = Vec::with_capacity(22);

        for motion_data in &self.car_motion_data {
            car_motion_data.push(CarMotionData {
                world_position_x: motion_data.world_position_x,
                world_position_y: motion_data.world_position_y,
                world_position_z: motion_data.world_position_z,
                world_velocity_x: motion_data.world_velocity_x,
                world_velocity_y: motion_data.world_velocity_y,
                world_velocity_z: motion_data.world_velocity_z,
                world_forward_dir_x: motion_data.world_forward_dir_x as i32,
                world_forward_dir_y: motion_data.world_forward_dir_y as i32,
                world_forward_dir_z: motion_data.world_forward_dir_z as i32,
                world_right_dir_x: motion_data.world_right_dir_x as i32,
                world_right_dir_y: motion_data.world_right_dir_y as i32,
                world_right_dir_z: motion_data.world_right_dir_z as i32,
                yaw: motion_data.yaw,
            });
        }

        Some(PacketMotionData { car_motion_data })
    }
}
