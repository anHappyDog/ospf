pub mod handle;
pub mod event;
pub mod status;


use core::net;
use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;




lazy_static::lazy_static! {
    pub static ref INTERFACE_STATUS_MAP: Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<status::Status>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref INTERFACE_MAP: Arc<RwLock<HashMap<net::Ipv4Addr,Interface>>> = Arc::new(RwLock::new(HashMap::new()));

}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetworkType {
    Broadcast,
    PointToPoint,
    NBMA,
    PointToMultipoint,
    VirtualLink,
}

unsafe impl Send for NetworkType {}

impl std::fmt::Debug for NetworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkType::Broadcast => write!(f, "Broadcast"),
            NetworkType::PointToPoint => write!(f, "PointToPoint"),
            NetworkType::NBMA => write!(f, "NBMA"),
            NetworkType::PointToMultipoint => write!(f, "PointToMultipoint"),
            NetworkType::VirtualLink => write!(f, "VirtualLink"),
        }
    }
}


pub struct Interface {
    pub ip: net::Ipv4Addr,
    pub mask: net::Ipv4Addr,
    pub area_id: net::Ipv4Addr,
    pub output_cost: u32,
    pub rxmt_interval: u32,
    pub inf_trans_delay: u32,
    pub hello_interval: u16,
    pub router_dead_interval: u32,
    pub network_type: NetworkType,
    pub auth_type: u16,
    pub auth_key: u64,
    pub options: u8,
    pub router_priority: u8,
}
