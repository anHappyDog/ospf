pub mod event;
pub mod handle;
pub mod status;
pub mod trans;
use core::net;
use std::{collections::HashMap, fmt::Debug, sync::Arc};
use tokio::sync::{Mutex, RwLock};

use crate::neighbor;

pub enum NetworkType {
    Broadcast,
    PointToPoint,
    NBMA,
    PointToMultipoint,
    VirtualLink,
}

impl Debug for NetworkType {
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
    pub area_id: u32,
    pub output_cost: u32,
    pub rxmt_interval: u32,
    pub inf_trans_delay: u32,
    pub hello_interval: u32,
    pub router_dead_interval: u32,
    pub network_type: NetworkType,
    pub auth_type: u8,
    pub autn_key: u64,
}

impl Interface {

}

lazy_static::lazy_static! {
    pub static ref INTERFACES : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Interface>>>>> = Arc::new(RwLock::new(HashMap::new()));
}


/// this function will init all the global data about interface
/// like the interfaces, neighbors, handlers
pub async fn init() {

}