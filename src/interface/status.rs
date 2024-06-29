use std::{fmt::Debug, net};

use tokio::sync::broadcast;

use super::handle::create_router_lsa;


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


/// the status machine of the passed param ipv4_addr 's interface
pub async fn changed(iaddr: net::Ipv4Addr) -> () {
    crate::util::debug("interface status machine started.");
    let mut event_rx = {
        let mut event_senders = super::event::EVENT_SENDERS.write().await;
        let (event_tx, _) = broadcast::channel(32);
        event_senders.insert(iaddr, event_tx.clone());
        event_tx.subscribe()
    };
    loop {
        match event_rx.recv().await {
            Ok(event) => match event {
                super::event::Event::InterfaceUp => {
                    crate::util::debug("interface up event received.");
                    super::event::Event::interface_up(iaddr).await;
                }
                super::event::Event::WaitTimer | super::event::Event::BackupSeen => {
                    crate::util::debug("wait timer event received.");
                    super::event::Event::wait_timer(iaddr).await;
                }
                super::event::Event::InterfaceDown => {
                    crate::util::debug("interface down event received.");
                    super::event::Event::interface_down(iaddr).await;
                }
                super::event::Event::LoopInd(_interface_name) => {
                    crate::util::debug("loop event received.");
                    super::event::Event::loop_ind(iaddr).await;
                }
                super::event::Event::NeighborChange(naddr) => {
                    crate::util::debug("neighbor change event received.");
                    super::event::Event::neighbor_change(iaddr,naddr).await;
                }
                super::event::Event::UnloopInd => {
                    crate::util::debug("unloop event received.");
                    super::event::Event::unloop_ind(iaddr).await;
                }
                _ => {
                    crate::util::error("invalid event received,ignored.");
                }
            },
            Err(_) => {
                continue;
            }
        }
        tokio::spawn(create_router_lsa(iaddr));
    }
}
