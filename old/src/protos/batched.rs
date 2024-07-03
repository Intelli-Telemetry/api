use ntex::util::{Bytes, BytesMut};
use prost::Message;

use super::{ChunkPacketHeader, PacketHeader};

pub struct ToProtoMessageBatched;

impl ToProtoMessageBatched {
    #[inline(always)]
    pub fn batched_encoded(packets: Vec<PacketHeader>) -> Option<Bytes> {
        let data = ChunkPacketHeader { packets };
        let mut buf = BytesMut::with_capacity(data.encoded_len());

        data.encode_raw(&mut buf);
        Some(buf.freeze())
    }
}
