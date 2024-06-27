use std::{collections::HashMap, fmt::Debug, net, sync::Arc};

use tokio::{
    runtime::Handle,
    sync::{broadcast, RwLock},
};
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Event {
    InterfaceUp,
    WaitTimer,
    BackupSeen,
    NeighborChange,
    LoopInd(String),
    UnloopInd,
    InterfaceDown,
}

impl Event {
    pub async fn interface_up(iaddr: net::Ipv4Addr) {
        let old_status = super::get_status(iaddr).await;
        if old_status != super::status::Status::Down {
            crate::util::error("interface is not down.");
            return;
        }
        let g_handlers = super::handle::HANDLE_MAP.read().await;
        let handler = g_handlers.get(&iaddr).unwrap();
        let mut locked_handler = handler.write().await;

        let new_status = locked_handler.when_interface_up(iaddr).await;
        super::set_status(iaddr, new_status);
    }
    pub async fn wait_timer(iaddr: net::Ipv4Addr) {
        let old_status = super::get_status(iaddr).await;
        if old_status != super::status::Status::Waiting {
            crate::util::error("interface is not waiting.,wait_timer event ignored.");
            return;
        }
        let new_status = select_dr_bdr().await;
        super::set_status(iaddr, new_status);
    }
    pub async fn backup_seen(iaddr: net::Ipv4Addr) {
        let old_status = super::get_status(iaddr).await;
        if old_status != super::status::Status::Waiting {
            crate::util::error("interface is not waiting.,backup_seen event ignored.");
            return;
        }
        let new_status = select_dr_bdr().await;
        super::set_status(iaddr, new_status);
    }
    pub async fn neighbor_change(iaddr: net::Ipv4Addr) {
        let old_status = super::get_status(iaddr).await;
        if old_status != super::status::Status::DR
            && old_status != super::status::Status::DRother
            && old_status != super::status::Status::Backup
        {
            crate::util::error(
                "interface is not dr,drother,backup ,neighbor_change event ignored.",
            );
            return;
        }
        let new_status = select_dr_bdr().await;
        super::set_status(iaddr, new_status);
        //TODO: HERE TO START THE DD NEOGOIATION
    }
    pub async fn loop_ind(iaddr: net::Ipv4Addr) {
        let g_handlers = super::handle::HANDLE_MAP.read().await;
        let handler = g_handlers.get(&iaddr).unwrap();
        let mut locked_handler = handler.write().await;
        tokio::join!(
            locked_handler.when_interface_down(iaddr),
            super::send_neighbor_killnbr(iaddr)
        );
        super::set_status(iaddr, super::status::Status::Loopback).await;
    }
    pub async fn unloop_ind(iaddr: net::Ipv4Addr) {
        let old_status = super::get_status(iaddr).await;
        if old_status != super::status::Status::Loopback {
            crate::util::error("interface is not loopback.,unloop_ind event ignored.");
            return;
        }
        super::set_status(iaddr, super::status::Status::Down).await;
    }
    pub async fn interface_down(iaddr: net::Ipv4Addr) {
        let g_handlers = super::handle::HANDLE_MAP.read().await;
        let handler = g_handlers.get(&iaddr).unwrap();
        let mut locked_handler = handler.write().await;
        tokio::join!(
            locked_handler.when_interface_down(iaddr),
            super::send_neighbor_killnbr(iaddr)
        );
        super::set_status(iaddr, super::status::Status::Down).await;
    }
}

unsafe impl Send for Event {}

pub async fn select_dr_bdr() -> super::status::Status {
    unimplemented!()
}

impl Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::InterfaceUp => write!(f, "Interface Up"),
            Event::WaitTimer => write!(f, "WaitTimer"),
            Event::BackupSeen => write!(f, "BackupSeen"),
            Event::NeighborChange => write!(f, "NeighborChange"),
            Event::LoopInd(_) => write!(f, "LoopInd"),
            Event::UnloopInd => write!(f, "UnloopInd"),
            Event::InterfaceDown => write!(f, "InterfaceDown"),
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
        let name_map = super::NAME_MAP.read().await;
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
