use std::{fmt::Debug, net};

use pnet::{
    packet::ip::IpNextHeaderProtocols::{Tcp, Udp},
    transport,
};
use tokio::sync::broadcast;

use crate::{neighbor, IPV4_PACKET_MTU};

use super::handle::HANDLERS;

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
    // not complete
    let g_interfaces = super::INTERFACES.read().await;
    let interface = g_interfaces.get(&ipv4_addr).unwrap();
    let locked_interface = interface.read().await;
    let network_type = locked_interface.network_type;
    let router_priority = locked_interface.router_priority;
    let dr = locked_interface.designated_router;
    let bdr = locked_interface.backup_designated_router;
    let (mut cur_rtr_pri, mut cur_dr, mut cur_bdr) = (router_priority, dr, bdr);
    let mut cur_router_id = net::Ipv4Addr::new(0, 0, 0, 0);
    let mut is_bdr_selected = false;
    let mut is_dr_selected = false;
    match network_type {
        _ => {
            // SOME NETWORK TYPES NO NEED TO SELECT THE DR AND BDR
        }
    }
    let g_neighbors = neighbor::NEIGHBORS.read().await;
    let neighbors = g_neighbors.get(&ipv4_addr).unwrap();
    let locked_neighbors = neighbors.read().await;
    for (neighbor_ipv4_addr, neighbor) in locked_neighbors.iter() {
        let locked_neighbor = neighbor.read().await;
        if locked_neighbor.priority <= cur_rtr_pri
            || locked_neighbor.dr == locked_neighbor.ipv4_addr
            || locked_neighbor.state < neighbor::status::Status::TwoWay
        {
            continue;
        }
        if locked_neighbor.priority > cur_rtr_pri || (locked_neighbor.priority == cur_rtr_pri && locked_neighbor.id > cur_router_id) {
            cur_rtr_pri = locked_neighbor.priority;
            cur_bdr = locked_neighbor.bdr;
            cur_router_id = neighbor_ipv4_addr.clone();
            is_bdr_selected = true;
        } 
    }
    if locked_interface.router_priority >= cur_rtr_pri ||( locked_interface.router_priority == cur_rtr_pri && crate::ROUTER_ID.clone() > cur_router_id) {
        cur_bdr = locked_interface.backup_designated_router;
        is_dr_selected = true;
        cur_router_id = crate::ROUTER_ID.clone();
        cur_rtr_pri = locked_interface.router_priority;
    }
    drop(locked_interface);
    let mut locked_interface = interface.write().await;
    locked_interface.backup_designated_router = cur_bdr;
    drop(locked_interface);
    
    cur_rtr_pri = 0;
    cur_router_id = net::Ipv4Addr::new(0, 0, 0, 0);



    // calcuate the backup designated router

    Status::DRother
}

/// the status machine of the passed param ipv4_addr 's interface
pub async fn changed(ipv4_addr: net::Ipv4Addr) -> () {
    let mut event_rx = {
        let mut event_senders = super::event::EVENT_SENDERS.write().await;
        let (event_tx, event_rx) = broadcast::channel(32);
        event_senders.insert(ipv4_addr, event_tx);
        event_rx
    };
    loop {
        match event_rx.recv().await {
            Ok(event) => match event {
                super::event::Event::InterfaceUp(interface_name) => {
                    let interfaces = super::INTERFACES.read().await;
                    let interface = interfaces.get(&ipv4_addr).unwrap();
                    let locked_interface = interface.read().await;
                    if locked_interface.status != Status::Down {
                        crate::util::error("interface status is not down,can't turn up.");
                        return;
                    }
                    drop(locked_interface);
                    drop(interfaces);
                    let (tcp_tx, tcp_rx) = match transport::transport_channel(
                        IPV4_PACKET_MTU,
                        transport::TransportChannelType::Layer3(Tcp),
                    ) {
                        Ok((tx, rx)) => (tx, rx),
                        Err(e) => {
                            crate::util::error(&format!("create tcp channel failed:{}", e));
                            return;
                        }
                    };
                    let (udp_tx, udp_rx) = match transport::transport_channel(
                        IPV4_PACKET_MTU,
                        transport::TransportChannelType::Layer3(Udp),
                    ) {
                        Ok((tx, rx)) => (tx, rx),
                        Err(e) => {
                            crate::util::error(&format!("create udp channel failed:{}", e));
                            return;
                        }
                    };
                    let global_handlers = HANDLERS.read().await;
                    let handler = match global_handlers.get(&ipv4_addr) {
                        Some(handler) => handler.clone(),
                        None => {
                            crate::util::error("handler not found.");
                            return;
                        }
                    };
                    let mut locked_handler = handler.write().await;
                    let global_trans = super::trans::TRANSMISSIONS.read().await;
                    let trans = global_trans.get(&ipv4_addr).unwrap();

                    locked_handler.send_tcp = Some(tokio::spawn(super::handle::send_tcp(
                        tcp_tx,
                        trans.inner_tcp_tx.subscribe(),
                    )));
                    locked_handler.send_udp = Some(tokio::spawn(super::handle::send_udp(
                        udp_tx,
                        trans.inner_udp_tx.subscribe(),
                    )));
                    locked_handler.recv_tcp = Some(tokio::spawn(super::handle::recv_tcp(tcp_rx)));
                    locked_handler.recv_udp = Some(tokio::spawn(super::handle::recv_udp(
                        udp_rx,
                        interface_name.clone(),
                        ipv4_addr.clone(),
                    )));
                    locked_handler.hello_timer = Some(tokio::spawn(super::handle::hello_timer(
                        ipv4_addr,
                        trans.inner_udp_tx.clone(),
                    )));
                    drop(global_trans);
                    drop(global_handlers);
                    let g_interfaces = super::INTERFACES.read().await;
                    let interface = g_interfaces.get(&ipv4_addr).unwrap();
                    let locked_interface = interface.read().await;
                    let network_type = locked_interface.network_type;
                    let router_priority = locked_interface.router_priority;
                    drop(locked_interface);
                    drop(g_interfaces);
                    match network_type {
                        super::NetworkType::Broadcast | super::NetworkType::NBMA => {
                            let g_interfaces = super::INTERFACES.read().await;
                            let interface = g_interfaces.get(&ipv4_addr).unwrap();
                            let mut locked_interface = interface.write().await;
                            locked_interface.status = Status::Waiting;
                            let router_dead_interval = locked_interface.router_dead_interval;
                            drop(locked_interface);
                            drop(g_interfaces);
                            if router_priority != 0 {
                                let handlers = HANDLERS.read().await;
                                let handler = handlers.get(&ipv4_addr).unwrap();
                                let mut locked_handler = handler.write().await;
                                locked_handler.wait_timer = Some(tokio::spawn(
                                    super::handle::wait_timer(ipv4_addr, router_dead_interval),
                                ));
                                drop(locked_handler);
                                drop(handlers);
                            }
                        }
                        _ => {
                            let g_interfaces = super::INTERFACES.read().await;
                            let interface = g_interfaces.get(&ipv4_addr).unwrap();
                            let mut locked_interface = interface.write().await;
                            locked_interface.status = Status::PointToPoint;
                            drop(locked_interface);
                            drop(g_interfaces);
                        }
                    }
                    drop(locked_handler);
                }
                super::event::Event::WaitTimer | super::event::Event::BackupSeen => {
                    let g_interfaces = super::INTERFACES.read().await;
                    let interface = g_interfaces.get(&ipv4_addr).unwrap();
                    let locked_interface = interface.read().await;
                    if locked_interface.status != Status::Waiting {
                        crate::util::error(
                            "interface status is not waiting,can't start wait timer.",
                        );
                        return;
                    }
                    drop(locked_interface);
                    drop(g_interfaces);
                    let new_status = select_dr_bdr(ipv4_addr).await;
                    let g_interfaces = super::INTERFACES.read().await;
                    let interface = g_interfaces.get(&ipv4_addr).unwrap();
                    let mut locked_interface = interface.write().await;
                    locked_interface.status = new_status;
                    drop(locked_interface);
                    drop(g_interfaces);
                    crate::util::debug(&format!("interface status changed to {:?}", new_status));
                    // here we should process the database exchange.
                }
                super::event::Event::InterfaceDown(interface_name) => {
                    let ipv4_addr = {
                        let interfaces_name_map = super::IPV4_NAME_MAP.read().await;
                        match interfaces_name_map.get(&interface_name) {
                            Some(ipv4_addr) => ipv4_addr.clone(),
                            None => {
                                crate::util::error("interface name not found.");
                                return;
                            }
                        }
                    };
                    let global_handlers = HANDLERS.read().await;
                    let handler = match global_handlers.get(&ipv4_addr) {
                        Some(handler) => handler.clone(),
                        None => {
                            crate::util::error("handler not found.");
                            return;
                        }
                    };
                    let mut locked_handler = handler.write().await;
                    if let Some(send_tcp) = locked_handler.send_tcp.take() {
                        send_tcp.abort();
                    }
                    if let Some(send_udp) = locked_handler.send_udp.take() {
                        send_udp.abort();
                    }
                    if let Some(recv_tcp) = locked_handler.recv_tcp.take() {
                        recv_tcp.abort();
                    }
                    if let Some(recv_udp) = locked_handler.recv_udp.take() {
                        recv_udp.abort();
                    }
                    if let Some(hello_timer) = locked_handler.hello_timer.take() {
                        hello_timer.abort();
                    }
                    if let Some(wait_timer) = locked_handler.wait_timer.take() {
                        wait_timer.abort();
                    }
                    drop(locked_handler);
                    drop(global_handlers);
                    let g_neighbors = super::neighbor::NEIGHBORS.read().await;
                    let neighbors = g_neighbors.get(&ipv4_addr).unwrap();
                    let mut locked_neighbors = neighbors.write().await;
                    for (neighbor_ipv4_addr, neighbor) in locked_neighbors.iter_mut() {
                        neighbor::event::send(
                            neighbor_ipv4_addr.clone(),
                            neighbor::event::Event::KillNbr,
                        )
                        .await;
                    }
                    locked_neighbors.clear();
                    drop(locked_neighbors);
                    drop(g_neighbors);
                    let g_interfaces = super::INTERFACES.read().await;
                    let interface = g_interfaces.get(&ipv4_addr).unwrap();
                    let mut locked_interface = interface.write().await;
                    locked_interface.status = Status::Down;
                }
                super::event::Event::LoopInd(interface_name) => {
                    let ipv4_addr = {
                        let interfaces_name_map = super::IPV4_NAME_MAP.read().await;
                        match interfaces_name_map.get(&interface_name) {
                            Some(ipv4_addr) => ipv4_addr.clone(),
                            None => {
                                crate::util::error("interface name not found.");
                                return;
                            }
                        }
                    };
                    let global_handlers = HANDLERS.read().await;
                    let handler = match global_handlers.get(&ipv4_addr) {
                        Some(handler) => handler.clone(),
                        None => {
                            crate::util::error("handler not found.");
                            return;
                        }
                    };
                    let mut locked_handler = handler.write().await;
                    if let Some(send_tcp) = locked_handler.send_tcp.take() {
                        send_tcp.abort();
                    }
                    if let Some(send_udp) = locked_handler.send_udp.take() {
                        send_udp.abort();
                    }
                    if let Some(recv_tcp) = locked_handler.recv_tcp.take() {
                        recv_tcp.abort();
                    }
                    if let Some(recv_udp) = locked_handler.recv_udp.take() {
                        recv_udp.abort();
                    }
                    if let Some(hello_timer) = locked_handler.hello_timer.take() {
                        hello_timer.abort();
                    }
                    if let Some(wait_timer) = locked_handler.wait_timer.take() {
                        wait_timer.abort();
                    }
                    drop(locked_handler);
                    drop(global_handlers);
                    let g_neighbors = super::neighbor::NEIGHBORS.read().await;
                    let neighbors = g_neighbors.get(&ipv4_addr).unwrap();
                    let mut locked_neighbors = neighbors.write().await;
                    for (neighbor_ipv4_addr, neighbor) in locked_neighbors.iter_mut() {
                        neighbor::event::send(
                            neighbor_ipv4_addr.clone(),
                            neighbor::event::Event::KillNbr,
                        )
                        .await;
                    }
                    locked_neighbors.clear();
                    drop(locked_neighbors);
                    drop(g_neighbors);
                    let g_interfaces = super::INTERFACES.read().await;
                    let interface = g_interfaces.get(&ipv4_addr).unwrap();
                    let mut locked_interface = interface.write().await;
                    locked_interface.status = Status::Loopback;
                }
                super::event::Event::NeighborChange => {
                    let g_interfaces = super::INTERFACES.read().await;
                    let interface = g_interfaces.get(&ipv4_addr).unwrap();
                    let locked_interface = interface.read().await;
                    if locked_interface.status != Status::DR
                        && locked_interface.status != Status::Backup
                        && locked_interface.status != Status::DRother
                    {
                        crate::util::error(
                            "interface status is not waiting,can't start wait timer.",
                        );
                        return;
                    }
                    drop(locked_interface);
                    drop(g_interfaces);
                    let new_status = select_dr_bdr(ipv4_addr).await;
                    let g_interfaces = super::INTERFACES.read().await;
                    let interface = g_interfaces.get(&ipv4_addr).unwrap();
                    let mut locked_interface = interface.write().await;
                    locked_interface.status = new_status;
                    drop(locked_interface);
                    drop(g_interfaces);
                    crate::util::debug(&format!("interface status changed to {:?}", new_status));
                    // here we should process the database exchange. only ajency
                }
                super::event::Event::UnloopInd => {
                    let g_interfaces = super::INTERFACES.read().await;
                    let interface = g_interfaces.get(&ipv4_addr).unwrap();
                    let mut locked_interface = interface.write().await;
                    if locked_interface.status != Status::Loopback {
                        crate::util::error("interface status is not loopback,can't unloop.");
                        return;
                    }
                    locked_interface.status = Status::Down;
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
