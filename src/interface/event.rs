use std::{collections::HashMap, fmt::Debug, net, sync::Arc};

use tokio::sync::{broadcast, RwLock};

use crate::neighbor;

use super::handle::start_dd_send;
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Event {
    InterfaceUp,
    WaitTimer,
    BackupSeen,
    NeighborChange(net::Ipv4Addr),
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
        super::set_status(iaddr, new_status).await;
    }
    pub async fn wait_timer(iaddr: net::Ipv4Addr) {
        let old_status = super::get_status(iaddr).await;
        if old_status != super::status::Status::Waiting {
            crate::util::error("interface is not waiting.,wait_timer event ignored.");
            return;
        }
        let new_status = select_dr_bdr(iaddr).await;
        super::set_status(iaddr, new_status).await;
    }
    pub async fn backup_seen(iaddr: net::Ipv4Addr) {
        let old_status = super::get_status(iaddr).await;
        if old_status != super::status::Status::Waiting {
            crate::util::error("interface is not waiting.,backup_seen event ignored.");
            return;
        }
        let new_status = select_dr_bdr(iaddr).await;
        super::set_status(iaddr, new_status).await;
    }
    pub async fn neighbor_change(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) {
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
        let new_status = select_dr_bdr(iaddr).await;
        super::set_status(iaddr, new_status).await;
        start_dd_send(iaddr, naddr, true, None).await;
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

pub async fn select_dr_bdr(iaddr: net::Ipv4Addr) -> super::status::Status {
    let mut cur_dr = super::get_dr(iaddr).await;
    let mut cur_bdr = super::get_bdr(iaddr).await;
    let mut cur_priority = 0;
    let mut bdr_selct_flag = false;
    let new_bdr;
    let mut max_priority = 0;
    let mut max_priority_id = net::Ipv4Addr::new(0, 0, 0, 0);
    let neighbors = neighbor::get_int_neighbors(iaddr).await;
    let locked_neighbors = neighbors.read().await;
    for (naddr, n) in &*locked_neighbors {
        let status = neighbor::get_status(iaddr, naddr.clone()).await;
        if status <= neighbor::status::Status::TwoWay {
            continue;
        }
        let locked_neighbor = n.read().await;
        if locked_neighbor.dr == locked_neighbor.id || locked_neighbor.priority == 0 {
            continue;
        }
        if locked_neighbor.bdr == locked_neighbor.id {
            if locked_neighbor.priority > cur_priority {
                cur_priority = locked_neighbor.priority;
                cur_bdr = locked_neighbor.id;
                bdr_selct_flag = true;
            } else if locked_neighbor.priority == cur_priority {
                if locked_neighbor.id > cur_bdr {
                    cur_bdr = locked_neighbor.id;
                    bdr_selct_flag = true;
                }
            }
        } else if !bdr_selct_flag {
            if locked_neighbor.priority > max_priority {
                max_priority = locked_neighbor.priority;
                max_priority_id = locked_neighbor.id;
            } else if locked_neighbor.priority == max_priority {
                if locked_neighbor.id > cur_dr {
                    cur_dr = locked_neighbor.id;
                    max_priority_id = locked_neighbor.id;
                }
            }
        }
    }
    if !bdr_selct_flag {
        super::set_bdr(iaddr, max_priority_id).await;
        new_bdr = max_priority_id;
    } else {
        super::set_bdr(iaddr, cur_bdr).await;
        new_bdr = cur_bdr;
    }
    cur_priority = 0;
    max_priority = 0;
    max_priority_id = net::Ipv4Addr::new(0, 0, 0, 0);
    bdr_selct_flag = false;
    // the bdr select is over.
    for (naddr, n) in &*locked_neighbors {
        let status = neighbor::get_status(iaddr, naddr.clone()).await;
        if status <= neighbor::status::Status::TwoWay {
            continue;
        }
        let locked_neighbor = n.read().await;
        if locked_neighbor.priority == 0 || locked_neighbor.id == new_bdr {
            continue;
        }
        if locked_neighbor.dr == locked_neighbor.id {
            if locked_neighbor.priority > cur_priority {
                cur_priority = locked_neighbor.priority;
                cur_dr = locked_neighbor.id;
            } else if locked_neighbor.priority == cur_priority {
                if locked_neighbor.id > cur_dr {
                    cur_dr = locked_neighbor.id;
                }
            }
        } else if !bdr_selct_flag {
            if locked_neighbor.priority > max_priority {
                max_priority = locked_neighbor.priority;
                max_priority_id = locked_neighbor.id;
            } else if locked_neighbor.priority == max_priority {
                if locked_neighbor.id > cur_dr {
                    max_priority_id = locked_neighbor.id;
                }
            }
        }
    }
    if !bdr_selct_flag {
        super::set_dr(iaddr, max_priority_id).await;
    } else {
        super::set_dr(iaddr, cur_dr).await;
    }
    if super::get_dr(iaddr).await == crate::ROUTER_ID.clone() {
        super::status::Status::DR
    } else if super::get_bdr(iaddr).await == crate::ROUTER_ID.clone() {
        super::status::Status::Backup
    } else {
        super::status::Status::DRother
    }
}

impl Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::InterfaceUp => write!(f, "Interface Up"),
            Event::WaitTimer => write!(f, "WaitTimer"),
            Event::BackupSeen => write!(f, "BackupSeen"),
            Event::NeighborChange(_) => write!(f, "NeighborChange"),
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
    let event_senders = EVENT_SENDERS.read().await;
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

pub async fn add_sender(ipv4_addr: net::Ipv4Addr) {
    let mut event_senders = EVENT_SENDERS.write().await;
    let (event_tx, _) = broadcast::channel(32);
    event_senders.insert(ipv4_addr, event_tx);
}

pub async fn remove_sender(ipv4_addr: net::Ipv4Addr) {
    let mut event_senders = EVENT_SENDERS.write().await;
    event_senders.remove(&ipv4_addr);
}
