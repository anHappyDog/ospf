use std::{collections::HashMap, net, sync::Arc};

use tokio::sync::{broadcast, RwLock};

/// # Transmission
/// the structure is used to store the senders for the interface
/// the network tx and rx is not included in the structure.
/// the inner_tcp_tx is used to send the message to the tcp handler
/// the inner_udp_tx is used to send the message to the udp handler
/// other interface or the inner interface can use this to forward
/// ipv4 packet.
pub struct Transmission {
    pub inner_tcp_tx: broadcast::Sender<bytes::Bytes>,
    pub inner_udp_tx: broadcast::Sender<bytes::Bytes>,
}

lazy_static::lazy_static! {
    /// the data structure is used to store all the transmissions for the interface
    /// the key is the ipv4 address of the interface
    pub static ref TRANSMISSIONS : Arc<RwLock<HashMap<net::Ipv4Addr, Arc<RwLock<Transmission>>>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub async fn init_transmissions(ipv4_addrs: Vec<net::Ipv4Addr>) {
    for ipv4_addr in ipv4_addrs {
        let (inner_tcp_tx, _) = broadcast::channel(1024);
        let (inner_udp_tx, _) = broadcast::channel(1024);
        let transmission = Transmission {
            inner_tcp_tx,
            inner_udp_tx,
        };
        TRANSMISSIONS
            .write()
            .await
            .insert(ipv4_addr, Arc::new(RwLock::new(transmission)));
    }
}
