use std::{collections::HashMap, fmt::Debug, net, sync::Arc};

use tokio::sync::{broadcast, RwLock};
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Event {
    InterfaceUp(String),
    WaitTimer,
    BackupSeen,
    NeighborChange,
    LoopInd(String),
    UnloopInd,
    InterfaceDown(String),
}

unsafe impl Send for Event {}

impl Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::InterfaceUp(_) => write!(f, "Interface Up"),
            Event::WaitTimer => write!(f, "WaitTimer"),
            Event::BackupSeen => write!(f, "BackupSeen"),
            Event::NeighborChange => write!(f, "NeighborChange"),
            Event::LoopInd(_) => write!(f, "LoopInd"),
            Event::UnloopInd => write!(f, "UnloopInd"),
            Event::InterfaceDown(_) => write!(f, "InterfaceDown"),
        }
    }
}

lazy_static::lazy_static! {
    pub(super) static ref EVENT_SENDERS: Arc<RwLock<HashMap<net::Ipv4Addr,broadcast::Sender<super::event::Event>>>> = Arc::new(RwLock::new(HashMap::new()));
}

/// # send_event
/// it will send the event to the according interface
/// - ipv4_addr : the interface 's ipv4 addr
/// - e : the event you want to send
pub async fn send(ipv4_addr: net::Ipv4Addr, e: Event) {
    let event_senders = EVENT_SENDERS.write().await;
    match event_senders.get(&ipv4_addr) {
        Some(sender) => match sender.send(e) {
            Ok(_) => {
                crate::util::debug("send event success.");
            }
            Err(_) => {
                crate::util::error("send event failed.");
            }
        },
        None => {
            crate::util::error("interface not found.");
        }
    }
}

pub async fn send_by_name(name: String, e: Event) {
    let ipv4_addr = {
        let name_map = super::IPV4_NAME_MAP.read().await;
        match name_map.get(&name) {
            Some(ipv4_addr) => ipv4_addr.clone(),
            None => {
                crate::util::error("interface name not found.");
                return;
            }
        }
    };
    send(ipv4_addr, e).await;
}

pub async fn add_sender(ipv4_addr: net::Ipv4Addr) {}

pub async fn remove_sender(ipv4_addr: net::Ipv4Addr) {}
