use std::{fmt::Debug, net};

use pnet::{
    packet::ip::IpNextHeaderProtocols::{Tcp, Udp},
    transport,
};
use tokio::sync::broadcast;

use crate::{neighbor, IPV4_PACKET_MTU};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Status {
    Down,
    Loopback,
    Waiting,
    PointToPoint,
    DRother,
    Backup,
    Question,
    DR,
}

impl Debug for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Down => write!(f, "Down"),
            Status::Loopback => write!(f, "Loopback"),
            Status::Waiting => write!(f, "Waiting"),
            Status::PointToPoint => write!(f, "PointToPoint"),
            Status::DRother => write!(f, "DRother"),
            Status::Backup => write!(f, "Backup"),
            Status::Question => write!(f, "Question"),
            Status::DR => write!(f, "DR"),
        }
    }
}

async fn select_dr_bdr(ipv4_addr: net::Ipv4Addr) -> Status {
    unimplemented!()
}

/// the status machine of the passed param ipv4_addr 's interface
pub async fn changed(iaddr: net::Ipv4Addr) -> () {
    let mut event_rx = {
        let mut event_senders = super::event::EVENT_SENDERS.write().await;
        let (event_tx, event_rx) = broadcast::channel(32);
        event_senders.insert(iaddr, event_tx);
        event_rx
    };
    loop {
        match event_rx.recv().await {
            Ok(event) => match event {
                super::event::Event::InterfaceUp => {
                    super::event::Event::interface_up(iaddr);
                }
                super::event::Event::WaitTimer | super::event::Event::BackupSeen => {
                    super::event::Event::wait_timer(iaddr);
                }
                super::event::Event::InterfaceDown => {
                    super::event::Event::interface_down(iaddr);
                }
                super::event::Event::LoopInd(interface_name) => {
                    super::event::Event::loop_ind(iaddr);
                }
                super::event::Event::NeighborChange => {
                    super::event::Event::neighbor_change(iaddr);
                }
                super::event::Event::UnloopInd => {
                    super::event::Event::unloop_ind(iaddr);
                }
                _ => {
                    crate::util::error("invalid event received,ignored.");
                }
            },
            Err(_) => {
                continue;
            }
        }
    }
}
