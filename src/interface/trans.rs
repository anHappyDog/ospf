use std::{collections::HashMap, net, sync::Arc};

use tokio::sync::{broadcast, RwLock};

use crate::packet::dd::DD;

pub const INNER_BUFFER_LENGTH: usize = 128;

pub struct Transmission {
    pub inner_dd_tx: broadcast::Sender<DD>,
    pub inner_lsr_tx: broadcast::Sender<bytes::Bytes>,
    pub inner_lsu_tx: broadcast::Sender<bytes::Bytes>,
}

lazy_static::lazy_static! {
    pub static ref TRANSMISSIONS : Arc<RwLock<HashMap<net::Ipv4Addr,Transmission>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref PACKET_SENDER : broadcast::Sender<bytes::Bytes> = init_packet_sender();
}

pub fn init_packet_sender() -> broadcast::Sender<bytes::Bytes> {
    let (tx, _) = broadcast::channel(1024);
    tx
}

pub async fn add(addr: net::Ipv4Addr) {
    let mut transmissions = TRANSMISSIONS.write().await;
    transmissions.insert(
        addr,
        Transmission {
            inner_dd_tx: broadcast::channel(INNER_BUFFER_LENGTH).0,
            inner_lsr_tx: broadcast::channel(INNER_BUFFER_LENGTH).0,
            inner_lsu_tx: broadcast::channel(INNER_BUFFER_LENGTH).0,
        },
    );
}

pub async fn init(addrs: Vec<net::Ipv4Addr>) {
    for addr in addrs {
        add(addr).await;
    }
}
