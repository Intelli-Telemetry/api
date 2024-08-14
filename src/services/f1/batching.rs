use std::sync::Arc;

use crate::{
    config::constants::{BATCHING_CAPACITY, BATCHING_INTERVAL},
    error::AppResult,
    protos::{batched::ToProtoMessageBatched, PacketHeader},
    structs::OptionalMessage,
    utils::compress_async,
};
use ntex::util::Bytes;
use parking_lot::{Mutex, RwLock};
use tokio::{
    sync::{broadcast::Sender, oneshot},
    time::interval,
};
use tracing::warn;

use super::caching::PacketCaching;

/// `PacketBatching` collects packets over 700ms, batches them together, and sends them as a single packet.
/// It also caches incoming packets for potential future use.
///
/// # Fields
///
/// - `shutdown`: Channel to signal the shutdown of the batching process.
/// - `cache`: Cache for additional packet storage.
///
/// # Functionality
///
/// Optimizes packet handling by batching multiple packets together, reducing overhead.
pub struct PacketBatching {
    packets: Arc<Mutex<Vec<PacketHeader>>>,
    cache: Arc<RwLock<PacketCaching>>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl PacketBatching {
    /// Creates a new `PacketBatching` instance.
    ///
    /// Initiates a background task that batches and sends data every 700ms, stopping
    /// when a shutdown signal is received.
    ///
    /// # Parameters
    ///
    /// - `tx`: Channel for sending batched data.
    /// - `cache`: Cache for storing packets.
    ///
    /// # Returns
    ///
    /// An instance ready to handle packet batching.
    pub fn new(tx: Sender<Bytes>, cache: Arc<RwLock<PacketCaching>>) -> Self {
        let (otx, mut orx) = oneshot::channel::<()>();
        let packets = Arc::from(Mutex::from(Vec::with_capacity(BATCHING_CAPACITY)));

        let instance = Self {
            packets: packets.clone(),
            shutdown: Some(otx),
            cache,
        };

        ntex::rt::spawn(async move {
            let mut interval_timer = interval(BATCHING_INTERVAL);

            loop {
                tokio::select! {
                    _ = interval_timer.tick() => {
                        if let Err(e) = Self::send_data(&packets, &tx).await {
                            warn!("Error while sending batched data: {}", e);
                        }
                    }

                    _ = (&mut orx) => {
                        break;
                    }
                }
            }
        });

        instance
    }

    /// Adds a `PacketHeader` to the buffer and caches it.
    ///
    /// # Parameters
    ///
    /// - `packet`: Packet to be added.
    #[inline(always)]
    pub fn push(&mut self, packet: PacketHeader) {
        self.push_with_optional_parameter(packet, None)
    }

    /// Adds a `PacketHeader` to the buffer with an optional parameter, and caches it.
    ///
    /// # Parameters
    ///
    /// - `packet`: Packet to be added.
    /// - `second_param`: Optional additional data for specific packet types.
    pub fn push_with_optional_parameter(
        &mut self,
        packet: PacketHeader,
        second_param: Option<OptionalMessage>,
    ) {
        let packet_type = packet.r#type();

        {
            let mut cache = self.cache.write();
            cache.save(packet_type, &packet.payload, second_param);
        }

        self.packets.lock().push(packet);
    }

    /// Sends batched packets from the buffer asynchronously.
    ///
    /// # Parameters
    ///
    /// - `buf`: Mutable reference to the packet buffer.
    /// - `tx`: Channel for sending the batched data.
    ///
    /// # Returns
    ///
    /// `AppResult<()>` indicating success or failure.
    #[inline(always)]
    async fn send_data(
        packets: &Arc<Mutex<Vec<PacketHeader>>>,
        tx: &Sender<Bytes>,
    ) -> AppResult<()> {
        let packets = {
            let mut packets = packets.lock();

            if packets.is_empty() {
                return Ok(());
            }

            let mut taken_buf = Vec::with_capacity(BATCHING_CAPACITY);
            std::mem::swap(&mut taken_buf, &mut *packets);

            taken_buf
        };

        if let Some(batched_packets) = ToProtoMessageBatched::batched_encoded(packets) {
            let encoded_packets = compress_async(batched_packets).await?;

            if tx.receiver_count() == 0 {
                return Ok(());
            }

            if let Err(e) = tx.send(encoded_packets) {
                warn!("Broadcast channel: {}", e);
            };
        }

        Ok(())
    }
}

impl Drop for PacketBatching {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}
