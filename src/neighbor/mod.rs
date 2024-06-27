use std::{collections::HashMap, net, sync::Arc};

use tokio::sync::RwLock;

pub mod event;
pub mod handle;
pub mod status;

// the key is the neighbors ipv4 address
lazy_static::lazy_static! {
    pub static ref NEIGHBOR_STATUS_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<status::Status>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref NEIGHBOR_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Neighbor>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref NEIHGBOR_LSA_RETRANS_LIST_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Vec<crate::lsa::Lsa>>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref NEIGHBOR_SUMMARY_LIST_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Vec<crate::lsa::Lsa>>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref NEIGHBOR_LSR_LIST_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Vec<crate::packet::lsr::Lsr>>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref NEIGHBOR_LAST_DD_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,crate::packet::dd::DD>>> = Arc::new(RwLock::new(HashMap::new()));

    // this key is the interface ipv4addr
    pub static ref INT_NEIGHBORS_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Vec<u32>>>>>> = Arc::new(RwLock::new(HashMap::new()));
}

#[derive(Clone,Copy)]
pub struct Neighbor {
    pub master: bool,
    pub dd_seq: u32,
    pub id: net::Ipv4Addr,
    pub priority: u8,
    pub ipv4_addr: net::Ipv4Addr,
    pub options: u8,
    pub dr: net::Ipv4Addr,
    pub bdr: net::Ipv4Addr,
}

impl Neighbor {}

pub async fn add(addr: net::Ipv4Addr, neighbor: Neighbor) {
    let mut neighbor_map = NEIGHBOR_MAP.write().await;
    neighbor_map.insert(addr, Arc::new(RwLock::new(neighbor)));
    drop(neighbor_map);
    let mut neighbor_status_map = NEIGHBOR_STATUS_MAP.write().await;
    neighbor_status_map.insert(addr, Arc::new(RwLock::new(status::Status::Down)));
    drop(neighbor_status_map);
    let mut neighbor_lsa_retrans_list_map = NEIHGBOR_LSA_RETRANS_LIST_MAP.write().await;
    neighbor_lsa_retrans_list_map.insert(addr, Arc::new(RwLock::new(Vec::new())));
    drop(neighbor_lsa_retrans_list_map);
    let mut neighbor_summary_list_map = NEIGHBOR_SUMMARY_LIST_MAP.write().await;
    neighbor_summary_list_map.insert(addr, Arc::new(RwLock::new(Vec::new())));
    drop(neighbor_summary_list_map);
    let mut neighbor_lsr_list_map = NEIGHBOR_LSR_LIST_MAP.write().await;
    neighbor_lsr_list_map.insert(addr, Arc::new(RwLock::new(Vec::new())));
    drop(neighbor_lsr_list_map);
    let mut handle_map = handle::HANDLE_MAP.write().await;
    let naddr = neighbor.ipv4_addr.clone();
    handle_map.insert(addr, handle::Handle::new(naddr, addr));
    drop(handle_map);
}
