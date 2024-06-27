use std::net;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

lazy_static::lazy_static! {
    // THE KEY IS THE AREA ID
    pub static ref AREA_MAP: Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Area>>>>> = Arc::new(RwLock::new(HashMap::new()));
    // REMEMBER THE LSDB IN THE AREA.
    pub static ref LSDB_MAP: Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<u32>>>>> = Arc::new(RwLock::new(HashMap::new()));
    // THE AREA'S CURRENT DR
    pub static ref DR_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,net::Ipv4Addr>>> = Arc::new(RwLock::new(HashMap::new()));
    // THE AREA'S CURRENT BDR
    pub static ref BDR_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,net::Ipv4Addr>>> = Arc::new(RwLock::new(HashMap::new()));

}

pub struct Area {
    pub id: net::Ipv4Addr,
    pub addr_range_list: Vec<AddrRange>,
    pub advertise_or_not: bool,
    pub external_routing_capability: bool,
}

pub struct AddrRange {
    pub addr: net::Ipv4Addr,
    pub mask: net::Ipv4Addr,
}

pub async fn create(area_id: net::Ipv4Addr) {}
