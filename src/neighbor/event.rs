use std::{collections::HashMap, net, sync::Arc};

use tokio::sync::{broadcast, RwLock};

use crate::{interface, lsa::summary};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Event {
    HelloReceived,
    Start,
    TwoWayReceived,
    NegotiationDone,
    ExchangeDone,
    BadLSReq,
    LoadingDone,
    AdjOk,
    SeqNumberMismatch,
    OneWayReceived,
    KillNbr,
    InactivityTimer,
    LLDown,
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::HelloReceived => write!(f, "HelloReceived"),
            Event::Start => write!(f, "Start"),
            Event::TwoWayReceived => write!(f, "TwoWayReceived"),
            Event::NegotiationDone => write!(f, "NegotiationDone"),
            Event::ExchangeDone => write!(f, "ExchangeDone"),
            Event::BadLSReq => write!(f, "BadLSReq"),
            Event::LoadingDone => write!(f, "LoadingDone"),
            Event::AdjOk => write!(f, "AdjOk"),
            Event::SeqNumberMismatch => write!(f, "SeqNumberMismatch"),
            Event::OneWayReceived => write!(f, "OneWayReceived"),
            Event::KillNbr => write!(f, "KillNbr"),
            Event::InactivityTimer => write!(f, "InactivityTimer"),
            Event::LLDown => write!(f, "LLDown"),
        }
    }
}

lazy_static::lazy_static! {
    pub static ref EVENT_SENDERS : Arc<RwLock<HashMap<net::Ipv4Addr,broadcast::Sender<super::event::Event>>>> = Arc::new(RwLock::new(HashMap::new()));
}

impl Event {
    pub async fn hello_received(naddr: net::Ipv4Addr) {
        let old_status = super::get_status(naddr).await;
        if old_status == super::status::Status::Down {
            //TODO  reset the inactive timer
            super::set_status(naddr, super::status::Status::Init).await;
        } else if old_status == super::status::Status::Attempt {
            //TODO reset the inactive timer
            super::set_status(naddr, super::status::Status::Init).await;
        } else if old_status >= super::status::Status::Init {
            //TOOD reset the inactive timer
        } else {
            crate::util::error("hello_received: invalid status,ignored.");
        }
    }
    pub async fn start(naddr: net::Ipv4Addr, iaddr: net::Ipv4Addr) {
        let network_type = {
            let interfaces_map = crate::interface::INTERFACE_MAP.read().await;
            let interface = interfaces_map.get(&iaddr).unwrap();
            interface.network_type
        };
        if interface::NetworkType::NBMA != network_type {
            crate::util::error("start: invalid network type,ignored.");
            return;
        }
        let old_status = super::get_status(naddr).await;
        if super::status::Status::Down != old_status {
            crate::util::error("start: invalid status,ignored.");
            return;
        }
        //TODO reset the inactive timer
        super::set_status(naddr, super::status::Status::Attempt).await;
    }
    pub async fn two_way_received(naddr: net::Ipv4Addr, iaddr: net::Ipv4Addr) {
        let old_status = super::get_status(naddr).await;
        if super::status::Status::Init == old_status {
            //TODO judge if need to build the adjacency,if not,turn to 2-way
            // otherwise, turn to Exstart.
            if true {
                // need to build the adjacency

                super::set_status(naddr, super::status::Status::ExStart).await;
            } else {
                // do not need to build the adjacency
                super::set_status(naddr, super::status::Status::TwoWay).await;
            }
        } else if super::status::Status::TwoWay <= old_status {
            crate::util::debug("two_way_received: already in 2-way,ignored.");
        } else {
            crate::util::error("two_way_received: invalid status,ignored.");
        }
    }
    pub async fn negotiation_done(naddr: net::Ipv4Addr, iaddr: net::Ipv4Addr) {}
    pub async fn exchange_done(naddr: net::Ipv4Addr, iaddr: net::Ipv4Addr) {}
    pub async fn bad_ls_req(naddr: net::Ipv4Addr, iaddr: net::Ipv4Addr) {
        let old_status = super::get_status(naddr).await;
        if super::status::Status::Exchange <= old_status {
            super::set_status(naddr, super::status::Status::ExStart).await;
            //TODO clear the three lists of the neighbor,and restart to dd negotiation
        } else {
            crate::util::error("bad_ls_req: invalid status,ignored.");
        }
    }
    pub async fn loading_done(naddr: net::Ipv4Addr) {
        let old_status = super::get_status(naddr).await;
        if super::status::Status::Loading != old_status {
            crate::util::error("loading_done: invalid status,ignored.");
            return;
        }
        super::set_status(naddr, super::status::Status::Full).await;
    }
    pub async fn adj_ok(naddr: net::Ipv4Addr, iaddr: net::Ipv4Addr) {
        let old_status = super::get_status(naddr).await;
        if super::status::Status::TwoWay == old_status {
            //TODO decide whether need to build the adjacency
            if true {
                // build the adjacency
                super::set_status(naddr, super::status::Status::ExStart).await;
            } else {
                crate::util::debug("adj_ok: do not need to build the adjacency,ignored.");
            }
        } else if super::status::Status::ExStart <= old_status {
            //TODO decide whether need to build the adjacency
            if true {
                crate::util::debug("adj_ok: no need to destroy the adjacency.");
            } else {
                //TODO destroy the adjacency: clear the three lists of the neighbor
                super::set_status(naddr, super::status::Status::TwoWay).await;
            }
        } else {
            crate::util::error("adj_ok: invalid status,ignored.");
        }
    }
    pub async fn seq_number_mismatch(naddr: net::Ipv4Addr, iaddr: net::Ipv4Addr) {
        let old_status = super::get_status(naddr).await;
        if super::status::Status::Exchange <= old_status {
            super::set_status(naddr, super::status::Status::ExStart).await;
            //TODO clear the three lists of the neighbor,and restart to dd negotiation
        } else {
            crate::util::error("seq_number_mismatch: invalid status,ignored.");
        }
    }
    pub async fn one_way_received(naddr: net::Ipv4Addr) {
        let old_status = super::get_status(naddr).await;
        if super::status::Status::TwoWay <= old_status {
            crate::util::debug("one_way_received: already in 2-way or higher,ignored.");
        } else if super::status::Status::Init == old_status {
            crate::util::debug("one_way_received: in init,ignored.");
        } else {
            crate::util::error("one_way_received: invalid status,ignored.");
        }
    }
    pub async fn kill_nbr(naddr: net::Ipv4Addr) {
        //TODO clear the three lists of the neighbor,and abort the inactive timer
        super::set_status(naddr, super::status::Status::Down).await;
    }
    pub async fn inactivity_timer(naddr: net::Ipv4Addr) {
        //TODO clear the three lists of the neighbor
        super::set_status(naddr, super::status::Status::Down).await;
    }
    pub async fn ll_down(naddr: net::Ipv4Addr) {
        //TODO clear the three lists of the neighbor,and abort the inactive timer
        super::set_status(naddr, super::status::Status::Down).await;
    }
}

/// # send_event
/// send the event to the ipv4_addr which represents a neighbor
/// - ipv4_addr : the neighbor's ipv4 addr
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
            crate::util::error("neighbor not found.");
        }
    }
}

pub async fn add_sender(ipv4_addr: net::Ipv4Addr) {
    let mut event_senders = EVENT_SENDERS.write().await;
    event_senders.insert(ipv4_addr, broadcast::channel(128).0);
}
