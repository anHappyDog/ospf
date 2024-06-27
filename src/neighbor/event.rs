use std::{collections::HashMap, net, sync::Arc};

use tokio::sync::{broadcast, RwLock};

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
    pub async fn hello_received() {}
    pub async fn start() {}
    pub async fn two_way_received() {}
    pub async fn negotiation_done() {}
    pub async fn exchange_done() {}
    pub async fn bad_ls_req() {}
    pub async fn loading_done() {}
    pub async fn adj_ok() {}
    pub async fn seq_number_mismatch() {}
    pub async fn one_way_received() {}
    pub async fn kill_nbr() {}
    pub async fn inactivity_timer() {}
    pub async fn ll_down() {}
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
