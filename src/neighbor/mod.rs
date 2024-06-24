use std::{collections::HashMap, net, sync::Arc};

use tokio::{sync::RwLock, task::JoinHandle};

pub mod event;
pub mod handle;
pub mod status;

/// # Neighbor
/// the data structure is used to store the neighbor for the interface
pub struct Neighbor {
    pub state: status::Status,
    pub inactive_timer: Option<JoinHandle<()>>,
    pub master: bool,
    pub dd_seq: u32,
    pub last_dd: Option<u32>,
    pub id: net::Ipv4Addr,
    pub priority: u32,
    pub ipv4_addr: net::Ipv4Addr,
    pub options: u8,
    pub dr: net::Ipv4Addr,
    pub bdr: net::Ipv4Addr,
    /*
    pub lsa_list :
    pub db_summary_list :
    pub lsr_list :
     */
}

lazy_static::lazy_static! {
    /// NEIGHBORS is a data structure used to store the neighbors for the interface
    /// it uses the interface's ipv4_addr to index its neighbors.
    pub static ref NEIGHBORS : Arc<RwLock<HashMap<net::Ipv4Addr, Arc<RwLock<HashMap<net::Ipv4Addr,Neighbor>>>>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub async fn init_neighbors(ipv4_addrs: Vec<net::Ipv4Addr>) {
    for ipv4_addr in ipv4_addrs {
        NEIGHBORS
            .write()
            .await
            .insert(ipv4_addr, Arc::new(RwLock::new(HashMap::new())));
    }
}

pub async fn status_changed(ipv4_addr: net::Ipv4Addr, event: event::Event) {
    
}
