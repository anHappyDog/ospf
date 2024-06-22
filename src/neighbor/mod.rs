use core::net;

use tokio::task::JoinHandle;

pub mod event;
pub mod status;

pub struct Neighbor {
    pub ipv4_addr: net::Ipv4Addr,
    pub status: status::NeighborStatus,
    pub dead_timer: Option<JoinHandle<()>>,
    pub neighbor_id: net::Ipv4Addr,
    pub neighbor_priority: u8,
    pub neighbor_designated_router: net::Ipv4Addr,
    pub neighbor_backup_designated_router: net::Ipv4Addr,
}

unsafe impl Send for Neighbor {}
unsafe impl Sync for Neighbor {}

impl Neighbor {
    pub fn change_to(&mut self, event: event::NeighborEvent) {
        
    }
}
