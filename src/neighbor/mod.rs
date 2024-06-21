use core::net;

use tokio::task::JoinHandle;

pub mod event;
pub mod status;

pub struct Neighbor {
    pub ipv4_addr: net::Ipv4Addr,
    pub status: status::NeighborStatus,
    pub dead_timer: Option<JoinHandle<()>>,
}

unsafe impl Send for Neighbor {}
unsafe impl Sync for Neighbor {}

impl Neighbor {
    pub fn new(ipv4_addr: net::Ipv4Addr) -> Self {
        Self {
            ipv4_addr,
            status: status::NeighborStatus::Down,
            dead_timer: None,
        }
    }
}
