use prost::Message;

use crate::protos::packet_header::PacketType;

include!(concat!(env!("OUT_DIR"), "/protos.packet_header.rs"));

pub(crate) mod batched;
pub(crate) mod car_motion_data;
pub(crate) mod event_data;
pub(crate) mod final_classification;
pub(crate) mod participants;
pub(crate) mod session_data;
pub(crate) mod session_history;

/// A trait for converting Rust data structures into their protobuf message equivalents.
///
/// This trait defines a generic way to transform a custom Rust type into a protobuf message,
/// which is useful for serializing data to be sent over the network or stored in a format
/// that is interoperable with other systems that use protobuf for serialization.
///
/// Types implementing this trait must specify their corresponding protobuf message type
/// and implement the `to_proto` method to perform the conversion. Additionally, a utility
/// method `convert` is provided to wrap the converted protobuf message into a `PacketHeader`,
/// preparing it for transmission by encapsulating the message with necessary metadata.
pub trait ToProtoMessage: Sized {
    /// The protobuf message type associated with this Rust type.
    type ProtoType: Message;

    /// Converts the implementing Rust type into its protobuf message equivalent.
    ///
    /// This method should encapsulate the logic necessary to transform the current state
    /// of the Rust object into a protobuf message. This may involve converting Rust-specific
    /// data types into types that are compatible with protobuf or flattening complex structures
    /// into a more serializable form.
    ///
    /// # Returns
    ///
    /// Returns `Option<Self::ProtoType>`, which is `Some(ProtoType)` if the conversion
    /// is successful, or `None` if the conversion cannot be performed (e.g., due to missing
    /// or invalid data).
    fn to_proto(&self) -> Option<Self::ProtoType>;

    /// Converts the implementing type into a `PacketHeader` containing the serialized protobuf message.
    ///
    /// This method leverages `to_proto` to convert the Rust type into its protobuf equivalent
    /// and then serializes that message into a byte vector. It wraps this byte vector into a
    /// `PacketHeader`, which includes metadata like the packet type. This is particularly useful
    /// for preparing the data for network transmission or storage.
    ///
    /// # Parameters
    ///
    /// - `packet_type`: The type of the packet, used to set the `type` field of the `PacketHeader`.
    ///
    /// # Returns
    ///
    /// Returns `Option<PacketHeader>`, which is `Some(PacketHeader)` if both the conversion to
    /// protobuf message and serialization are successful, or `None` if any step fails.
    ///
    /// # Examples
    ///
    /// Assuming `MyStruct` implements `ToProtoMessage`:
    /// ```
    /// let instance = MyStruct::new(...);
    /// let packet_type = PacketType::MyPacketType;
    /// let packet_header = instance.convert(packet_type);
    /// if let Some(header) = packet_header {
    ///     // Use `header` for further operations, like sending over the network.
    /// } else {
    ///     // Handle conversion or serialization failure.
    /// }
    /// ```
    fn convert(&self, packet_type: PacketType) -> Option<PacketHeader> {
        let proto_data = self.to_proto()?;
        let proto_data: Vec<u8> = proto_data.encode_to_vec();

        Some(PacketHeader {
            r#type: packet_type.into(),
            payload: proto_data,
        })
    }
}
