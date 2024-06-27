use std::net;

use rustyline::Event;
use tokio::sync::broadcast;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Down,
    Init,
    TwoWay,
    ExStart,
    Exchange,
    Loading,
    Full,
    Attempt,
}

pub async fn changed(naddr: net::Ipv4Addr, iaddr: net::Ipv4Addr) {
    let event_senders = crate::neighbor::event::EVENT_SENDERS.read().await;
    let mut event_rx: broadcast::Receiver<super::event::Event> =
        event_senders.get(&naddr).unwrap().subscribe();
    drop(event_senders);

    loop {
        match event_rx.recv().await {
            Ok(event) => match event {
                super::event::Event::HelloReceived => {
                    crate::util::debug(&format!("HelloReceived: {}", naddr));
                    super::event::Event::hello_received(naddr).await;
                }
                super::event::Event::Start => {
                    crate::util::debug(&format!("Start: {}", naddr));
                    super::event::Event::start(naddr, iaddr).await;
                }
                super::event::Event::TwoWayReceived => {
                    crate::util::debug(&format!("TwoWayReceived: {}", naddr));
                    super::event::Event::two_way_received(naddr, iaddr).await;
                }
                super::event::Event::NegotiationDone => {
                    crate::util::debug(&format!("NegotiationDone: {}", naddr));
                    super::event::Event::negotiation_done(naddr, iaddr).await;
                }
                super::event::Event::ExchangeDone => {
                    crate::util::debug(&format!("ExchangeDone: {}", naddr));
                    super::event::Event::exchange_done(naddr, iaddr).await;
                }
                super::event::Event::BadLSReq => {
                    crate::util::debug(&format!("BadLSReq: {}", naddr));
                    super::event::Event::bad_ls_req(naddr, iaddr).await;
                }
                super::event::Event::LoadingDone => {
                    crate::util::debug(&format!("LoadingDone: {}", naddr));
                    super::event::Event::loading_done(naddr).await;
                }
                super::event::Event::AdjOk => {
                    crate::util::debug(&format!("AdjOk: {}", naddr));
                    super::event::Event::adj_ok(naddr, iaddr).await;
                }
                super::event::Event::SeqNumberMismatch => {
                    crate::util::debug(&format!("SeqNumberMismatch: {}", naddr));
                    super::event::Event::seq_number_mismatch(naddr, iaddr).await;
                }
                super::event::Event::OneWayReceived => {
                    crate::util::debug(&format!("OneWayReceived: {}", naddr));
                    super::event::Event::one_way_received(naddr).await;
                }
                super::event::Event::InactivityTimer => {
                    crate::util::debug(&format!("InactivityTimer: {}", naddr));
                    super::event::Event::inactivity_timer(naddr).await;
                }
                super::event::Event::KillNbr => {
                    crate::util::debug(&format!("KillNbr: {}", naddr));
                    super::event::Event::kill_nbr(naddr).await;
                }
                super::event::Event::LLDown => {
                    crate::util::debug(&format!("LLDown: {}", naddr));
                    super::event::Event::ll_down(naddr).await;
                }
            },
            Err(e) => {
                crate::util::error(&format!("Error: {:?}", e));
            }
        }
    }
}
