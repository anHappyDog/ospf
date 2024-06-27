pub mod graph;
pub mod spt;

use std::net;

pub enum DestionationType {
    Network,
    Router,
}

pub enum PathType {
    IntraArea,
    InterArea,
    ExternalType1,
    ExternalType2,
}

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
