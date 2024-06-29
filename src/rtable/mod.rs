pub mod graph;
pub mod spt;
use std::net;
use std::sync::Arc;

use tokio::sync::RwLock;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DestionationType {
    Network,
    Router,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PathType {
    IntraArea,
    InterArea,
    ExternalType1,
    ExternalType2,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteTableEntry {
    pub destination_type: DestionationType,
    pub destination_id: net::Ipv4Addr,
    pub mask: net::Ipv4Addr,
    pub options: u8,
    pub area_id: u32,
    pub path_type: PathType,
    pub cost: u32,
    pub type2_cost: u32,
    pub ls_origin: u32,
    pub next_hop: net::Ipv4Addr,
    pub advertising_router: net::Ipv4Addr,
}

pub struct RouteTable {
    pub entries: Vec<RouteTableEntry>,
}

impl RouteTable {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    pub fn add_entry(&mut self, entry: RouteTableEntry) {
        self.entries.push(entry);
    }
    pub fn remove_entry(&mut self, entry: RouteTableEntry) {
        self.entries.retain(|e| e != &entry);
    }
}

lazy_static::lazy_static! {
    pub static ref ROUTE_TABLE : Arc<RwLock<RouteTable>> = Arc::new(RwLock::new(RouteTable::new()));
}

pub async fn update_route_table() {
    unimplemented!()
}
