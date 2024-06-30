use std::{collections::HashMap, net, sync::Arc};

use tokio::sync::{broadcast, RwLock};

use crate::packet::dd::DD;

pub const INNER_BUFFER_LENGTH: usize = 128;

pub struct Transmission {
    pub inner_dd_tx: broadcast::Sender<DD>,
    pub inner_lsr_tx: broadcast::Sender<bytes::Bytes>,
    pub inner_lsu_tx: broadcast::Sender<bytes::Bytes>,
    pub inner_packet_tx: broadcast::Sender<bytes::Bytes>,
}

lazy_static::lazy_static! {
    pub static ref TRANSMISSIONS : Arc<RwLock<HashMap<net::Ipv4Addr,Transmission>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub async fn get_packet_inner_tx(iaddr: net::Ipv4Addr) -> broadcast::Sender<bytes::Bytes> {
    let transmissions = TRANSMISSIONS.read().await;
    let transmission = transmissions.get(&iaddr).unwrap();
    transmission.inner_packet_tx.clone()
}

pub async fn get_packet_inner_rx(iaddr: net::Ipv4Addr) -> broadcast::Receiver<bytes::Bytes> {
    let transmissions = TRANSMISSIONS.read().await;
    let transmission = transmissions.get(&iaddr).unwrap();
    transmission.inner_packet_tx.subscribe()
}

pub async fn add(addr: net::Ipv4Addr) {
    let mut transmissions = TRANSMISSIONS.write().await;
    transmissions.insert(
        addr,
        Transmission {
            inner_dd_tx: broadcast::channel(INNER_BUFFER_LENGTH).0,
            inner_lsr_tx: broadcast::channel(INNER_BUFFER_LENGTH).0,
            inner_lsu_tx: broadcast::channel(INNER_BUFFER_LENGTH).0,
            inner_packet_tx: broadcast::channel(INNER_BUFFER_LENGTH).0,
        },
    );
}

pub async fn init(addrs: Vec<net::Ipv4Addr>) {
    for addr in addrs {
        add(addr).await;
    }
}
