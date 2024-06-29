use core::net;
use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::interface::{self};

lazy_static::lazy_static! {
    pub static ref HANDLE_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<HashMap<net::Ipv4Addr,Handle>>>>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub struct Handle {
    pub status_machine: Option<tokio::task::JoinHandle<()>>,
    pub inactive_timer: Option<tokio::task::JoinHandle<()>>,
    pub dd_negoiation: Option<tokio::task::JoinHandle<()>>,
    pub dd_exchange: Option<tokio::task::JoinHandle<()>>,
}

impl Handle {
    pub fn new(naddr: net::Ipv4Addr, iaddr: net::Ipv4Addr) -> Self {
        Self {
            status_machine: Some(tokio::spawn(super::status::changed(naddr, iaddr))),
            inactive_timer: None,
            dd_negoiation: None,
            dd_exchange: None,
        }
    }
}

pub async fn abort_inactive_timer(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) {
    let handle_list = HANDLE_MAP.read().await;
    let n_handle_list = handle_list.get(&iaddr).unwrap();
    let mut locked_handle_list = n_handle_list.write().await;
    let handle = locked_handle_list.get_mut(&naddr).unwrap();
    if let Some(inactive_timer) = &handle.inactive_timer {
        inactive_timer.abort();
    }
    handle.inactive_timer = None;
}

pub async fn start_inactive_timer(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) {
    abort_inactive_timer(iaddr, naddr).await;
    let handle_list = HANDLE_MAP.read().await;
    let n_handle_list = handle_list.get(&iaddr).unwrap();
    let mut locked_handle_list = n_handle_list.write().await;
    let handle = locked_handle_list.get_mut(&naddr).unwrap();
    handle.inactive_timer = Some(tokio::spawn(super::handle::inactive_timer(iaddr, naddr)));
}

pub async fn inactive_timer(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) {
    let router_dead_interval = interface::get_router_dead_interval(iaddr).await;
    tokio::time::sleep(std::time::Duration::from_secs(router_dead_interval as u64)).await;
    super::event::send(iaddr, naddr, super::event::Event::InactivityTimer).await;
}


