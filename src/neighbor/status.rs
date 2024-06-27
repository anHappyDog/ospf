use std::net;

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
                    super::event::Event::hello_received().await;
                }
                super::event::Event::Start => {
                    super::event::Event::start().await;
                }
                super::event::Event::TwoWayReceived => {
                    super::event::Event::two_way_received().await;
                }
                super::event::Event::NegotiationDone => {
                    super::event::Event::negotiation_done().await;
                }
                super::event::Event::ExchangeDone => {
                    super::event::Event::exchange_done().await;
                }
                super::event::Event::BadLSReq => {
                    super::event::Event::bad_ls_req().await;
                }
                super::event::Event::LoadingDone => {
                    super::event::Event::loading_done().await;
                }
                super::event::Event::AdjOk => {
                    super::event::Event::adj_ok().await;
                }
                super::event::Event::SeqNumberMismatch => {
                    super::event::Event::seq_number_mismatch().await;
                }
                super::event::Event::OneWayReceived => {
                    super::event::Event::one_way_received().await;
                }
                super::event::Event::KillNbr => {
                    super::event::Event::kill_nbr().await;
                }
                super::event::Event::InactivityTimer => {
                    super::event::Event::inactivity_timer().await;
                }
                super::event::Event::LLDown => {
                    super::event::Event::ll_down().await;
                }
            },
            Err(e) => {
                crate::util::error(&format!("Error: {:?}", e));
            }
        }
    }
}
