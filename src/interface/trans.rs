use std::{collections::HashMap, net, sync::Arc};

use tokio::sync::{broadcast, RwLock};

pub const INNER_BUFFER_LENGTH: usize = 128;

pub struct Transmission {
    pub inner_tcp_tx: broadcast::Sender<bytes::Bytes>,
    pub inner_udp_tx: broadcast::Sender<bytes::Bytes>,
    pub inner_dd_tx: broadcast::Sender<bytes::Bytes>,
    pub inner_lsr_tx: broadcast::Sender<bytes::Bytes>,
    pub inner_lsu_tx: broadcast::Sender<bytes::Bytes>,
}

lazy_static::lazy_static! {
    pub static ref TRANSMISSIONS : Arc<RwLock<HashMap<net::Ipv4Addr,Transmission>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub async fn add(addr: net::Ipv4Addr) {
    let mut transmissions = TRANSMISSIONS.write().await;
    transmissions.insert(
        addr,
        Transmission {
            inner_tcp_tx: broadcast::channel(INNER_BUFFER_LENGTH).0,
            inner_udp_tx: broadcast::channel(INNER_BUFFER_LENGTH).0,
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
