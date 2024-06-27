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
