use core::net;
use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

lazy_static::lazy_static! {
    pub static ref HANDLE_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Handle>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub struct Handle {
    pub status_machine: Option<tokio::task::JoinHandle<()>>,
}

impl Handle {
    pub fn new(naddr: net::Ipv4Addr, iaddr: net::Ipv4Addr) -> Self {
        Self {
            status_machine: Some(tokio::spawn(super::status::changed(naddr, iaddr))),
        }
    }
}
