use crate::lsa::router::{LinkState, RouterLSA, LS_ID_STUB, LS_ID_TRANSIT};
use crate::lsa::Lsa;
use crate::neighbor::{get_ddseqno, get_int_neighbors, save_last_dd};
use crate::packet::dd::{FLAG_I_BIT, FLAG_MS_BIT, FLAG_M_BIT};
use crate::packet::lsack::Lsack;
use crate::packet::lsu::Lsu;
use crate::{area, lsa, rtable, OPTION_E};
use pnet::datalink::Channel::Ethernet;
use pnet::datalink::{DataLinkReceiver, DataLinkSender};
use pnet::packet::ethernet::{self, EtherTypes};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::{
    datalink::{self, Config},
    packet::{
        ip::{self},
        ipv4, Packet,
    },
    transport,
};
use std::default;
use std::ops::BitAnd;
use std::{collections::HashMap, net, sync::Arc};
use tokio::task::JoinHandle;
use tokio::{sync::RwLock, time};

use crate::{neighbor, packet::dd::DD, IPV4_PACKET_MTU, OSPF_IP_PROTOCOL};

use super::{status, INTERFACE_MAP};

// ONLY USE IN THE INNER INTERFACE
// KEY IS THE INTERFACE 'S IPV4 ADDR
lazy_static::lazy_static! {
    pub static ref HANDLE_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Handle>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref PACKET_SEND : Arc<RwLock<Option<JoinHandle<()>>>> = Arc::new(RwLock::new(None));
}

pub async fn start_send_lsr(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) {
    let g_handles = HANDLE_MAP.read().await;
    let int_handles = g_handles.get(&iaddr).unwrap();
    let mut locked_int_handles = int_handles.write().await;
    if let Some(lsr_send) = &locked_int_handles.lsr_send {
        lsr_send.abort();
    }
    let lsr_send = tokio::spawn(send_lsr(iaddr, naddr));
    locked_int_handles.lsr_send = Some(lsr_send);
}

pub async fn stop_send_lsr(iaddr: net::Ipv4Addr) {
    let g_handles = HANDLE_MAP.read().await;
    let int_handles = g_handles.get(&iaddr).unwrap();
    let mut locked_int_handles = int_handles.write().await;
    if let Some(lsr_send) = &locked_int_handles.lsr_send {
        lsr_send.abort();
        locked_int_handles.lsr_send = None;
    }
}

pub async fn start_send_lsu(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) {
    let g_handles = HANDLE_MAP.read().await;
    let int_handles = g_handles.get(&iaddr).unwrap();
    let mut locked_int_handles = int_handles.write().await;
    if let Some(lsu_send) = &locked_int_handles.lsu_send {
        lsu_send.abort();
    }
    let lsu_send = tokio::spawn(send_lsu(iaddr, naddr));
    locked_int_handles.lsu_send = Some(lsu_send);
}

pub async fn stop_send_lsu(iaddr: net::Ipv4Addr) {
    let g_handles = HANDLE_MAP.read().await;
    let int_handles = g_handles.get(&iaddr).unwrap();
    let mut locked_int_handles = int_handles.write().await;
    if let Some(lsu_send) = &locked_int_handles.lsu_send {
        lsu_send.abort();
        locked_int_handles.lsu_send = None;
    }
}

pub async fn send_lsu(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) {
    let rxmt_interval = super::get_rxmt_interval(iaddr).await;
    let interval = time::Duration::from_secs(rxmt_interval as u64);
    let packet_inner_tx = super::trans::get_packet_inner_tx(iaddr).await;
    let retrans_list = neighbor::get_trans_list(iaddr, naddr).await;
    let mut buffer: Vec<u8> = vec![0; crate::IPV4_PACKET_MTU];
    let mut bf: Vec<u8> = vec![0; IPV4_PACKET_MTU + 100];
    loop {
        let locked_retrans_list = retrans_list.read().await;
        let lsas = match area::lsdb::fetch_lsas(iaddr, locked_retrans_list.clone()).await {
            Some(lsas) => lsas,
            None => {
                crate::util::error("can't get the lsa according to the neighbor's retrans list.");
                return;
            }
        };
        let lsu_packet = Lsu::new(iaddr, naddr, lsas.clone()).await;
        let ip_packet = Lsu::build_ipv4_packet(lsu_packet.clone(), &mut buffer, iaddr, naddr)
            .await
            .unwrap();
        let ether_packet = crate::packet::build_ether_packet(
            &mut bf,
            ip_packet,
            super::get_mac(iaddr).await,
            neighbor::get_mac(iaddr, naddr).await,
        )
        .await;
        match packet_inner_tx.send(bytes::Bytes::from(ether_packet.packet().to_vec())) {
            Ok(_) => {
                crate::util::debug("send lsu packet success.");
            }
            Err(e) => {
                crate::util::error(&format!("send lsu packet failed:{}", e));
                continue;
            }
        }
        time::sleep(interval).await;
    }
}

pub async fn start_send_lsack(
    iaddr: net::Ipv4Addr,
    naddr: net::Ipv4Addr,
    lsa_headers: Option<Vec<lsa::Header>>,
) {
    let g_handles = HANDLE_MAP.read().await;
    let int_handles = g_handles.get(&iaddr).unwrap();
    let mut locked_int_handles = int_handles.write().await;
    if let Some(lsack_send) = &locked_int_handles.lsack_send {
        lsack_send.abort();
    }
    let lsack_send = tokio::spawn(send_lsack(iaddr, naddr, lsa_headers));
    locked_int_handles.lsack_send = Some(lsack_send);
}

// the should be called when the interface's status is changed
pub async fn create_router_lsa(iaddr: net::Ipv4Addr) {
    let area_id = super::get_area_id(iaddr).await;
    let imap = INTERFACE_MAP.read().await;
    let mut links = Vec::new();
    for (iaddr, int) in &*imap {
        // you should judge whether the interface's network
        // belongs to the area.
        if int.area_id != area_id {
            continue;
        }
        let istatus = super::get_status(iaddr.clone()).await;
        let network_type = int.network_type;
        let mask = int.mask;
        let metric = int.output_cost;
        match istatus {
            super::status::Status::Down => {
                continue;
            }
            super::status::Status::Loopback => match network_type {
                super::NetworkType::PointToPoint => {
                    continue;
                }
                _ => {
                    links.push(LinkState::new(
                        iaddr.clone().into(),
                        net::Ipv4Addr::new(255, 255, 255, 255).into(),
                        LS_ID_STUB,
                        0,
                        None,
                        0,
                    ));
                }
            },
            _ => match network_type {
                super::NetworkType::PointToPoint => {
                    unimplemented!()
                }
                super::NetworkType::Broadcast | super::NetworkType::NBMA => {
                    match istatus {
                        super::status::Status::Waiting => {
                            links.push(LinkState::new(
                                iaddr.bitand(mask).into(), // get the network's ip address
                                mask.into(),
                                LS_ID_STUB,
                                0,
                                None,
                                metric as u16,
                            ));
                        }
                        _ => {
                            let dr_id = super::get_dr(iaddr.clone()).await;
                            if dr_id == net::Ipv4Addr::new(0, 0, 0, 0) {
                                links.push(LinkState::new(
                                    iaddr.bitand(mask).into(), // get the network's ip address
                                    mask.into(),
                                    LS_ID_STUB,
                                    0,
                                    None,
                                    metric as u16,
                                ));
                            } else if dr_id == crate::ROUTER_ID.clone() {
                                // check whether one neighbor is exstart or higher
                                let neighbors = get_int_neighbors(iaddr.clone()).await;
                                let mut flag = false;
                                let locked_neighbors = neighbors.read().await;
                                for (naddr, _) in locked_neighbors.iter() {
                                    if neighbor::get_status(iaddr.clone(), naddr.clone()).await
                                        >= neighbor::status::Status::ExStart
                                    {
                                        flag = true;
                                        break;
                                    }
                                }
                                if flag {
                                    links.push(LinkState::new(
                                        iaddr.clone().into(), // get the network's ip address
                                        iaddr.clone().into(),
                                        LS_ID_TRANSIT,
                                        0,
                                        None,
                                        metric as u16,
                                    ));
                                } else {
                                    links.push(LinkState::new(
                                        iaddr.bitand(mask).into(), // get the network's ip address
                                        mask.into(),
                                        LS_ID_STUB,
                                        0,
                                        None,
                                        metric as u16,
                                    ));
                                }
                            } else {
                                let nstatus =
                                    neighbor::get_status_by_id(iaddr.clone(), dr_id).await;
                                match nstatus {
                                    Some(nstatus) => {
                                        if nstatus == neighbor::status::Status::Full {
                                            let naddr =
                                                neighbor::get_naddr_by_id(iaddr.clone(), dr_id)
                                                    .await
                                                    .unwrap();
                                            links.push(LinkState::new(
                                                naddr.clone().into(),
                                                iaddr.clone().into(),
                                                LS_ID_TRANSIT,
                                                0,
                                                None,
                                                metric as u16,
                                            ));
                                        } else {
                                            links.push(LinkState::new(
                                                iaddr.bitand(mask).into(), // get the network's ip address
                                                mask.into(),
                                                LS_ID_STUB,
                                                0,
                                                None,
                                                metric as u16,
                                            ));
                                        }
                                    }
                                    None => {
                                        crate::util::error("can't get the dr's neighbor's status");
                                    }
                                }
                            }
                        }
                    }
                }
                super::NetworkType::PointToMultipoint => {
                    unimplemented!()
                }
                super::NetworkType::VirtualLink => {}
            },
        }
        // after process the interface, you should also process the
        // host that connected to  the area, but here we just ignore it.
    }
    let router_lsa = RouterLSA::new(links, OPTION_E).await;
    let lsas = vec![Lsa::Router(router_lsa)];
    area::lsdb::update_lsdb(iaddr, lsas).await;
}

pub async fn create_network_lsa(_iaddr: net::Ipv4Addr) {
    unimplemented!()
}

pub async fn create_summary_lsa(_iaddr: net::Ipv4Addr) {
    unimplemented!()
}

pub async fn stop_send_lsack(iaddr: net::Ipv4Addr) {
    let g_handles = HANDLE_MAP.read().await;
    let int_handles = g_handles.get(&iaddr).unwrap();
    let mut locked_int_handles = int_handles.write().await;
    if let Some(lsack_send) = &locked_int_handles.lsack_send {
        lsack_send.abort();
        locked_int_handles.lsack_send = None;
    }
}

pub async fn send_lsack(
    iaddr: net::Ipv4Addr,
    naddr: net::Ipv4Addr,
    lsa_headers: Option<Vec<lsa::Header>>,
) {
    let lsack_packet = Lsack::new(iaddr, naddr, lsa_headers.clone()).await;
    let mut buffer: Vec<u8> = vec![0; crate::IPV4_PACKET_MTU];
    let ip_packet = lsack_packet
        .build_ipv4_packet(&mut buffer, iaddr, naddr)
        .await
        .unwrap();
    let mut bf = vec![0; IPV4_PACKET_MTU + 100];
    let packet_inner_tx = super::trans::get_packet_inner_tx(iaddr).await;
    let source_mac = super::get_mac(iaddr).await;
    let neighbor_mac = neighbor::get_mac(iaddr, naddr).await;
    let ether_packet =
        crate::packet::build_ether_packet(&mut bf, ip_packet, source_mac, neighbor_mac).await;
    match packet_inner_tx.send(bytes::Bytes::from(ether_packet.packet().to_vec())) {
        Ok(_) => {
            crate::util::debug("send lsack packet success.");
        }
        Err(e) => {
            crate::util::error(&format!("send lsack packet failed:{}", e));
        }
    }
}

pub async fn send_lsr(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) {
    let g_lsr_list = neighbor::NEIGHBOR_LSR_LIST_MAP.read().await;
    let int_lsr_list = g_lsr_list.get(&iaddr).unwrap();
    let locked_int_lsr_list = int_lsr_list.read().await;
    let lsr_list = locked_int_lsr_list.get(&naddr).unwrap();
    let lsr_list = lsr_list.read().await;
    let rxmt_interval = super::get_rxmt_interval(iaddr).await;
    let interval = time::Duration::from_secs(rxmt_interval as u64);
    let packet_inner_tx = super::trans::get_packet_inner_tx(iaddr).await;
    let mut bf: Vec<u8> = vec![0; IPV4_PACKET_MTU + 100];
    let neighbor_mac = neighbor::get_mac(iaddr, naddr).await;
    let source_mac = super::get_mac(iaddr).await;
    loop {
        let lsr_packet = crate::packet::lsr::Lsr::new(iaddr, lsr_list.clone()).await;
        let mut buffer = vec![0; IPV4_PACKET_MTU];
        let ippacket = lsr_packet
            .build_ipv4_packet(&mut buffer, iaddr, naddr)
            .await
            .unwrap();
        let ether_packet =
            crate::packet::build_ether_packet(&mut bf, ippacket, source_mac, neighbor_mac).await;

        match packet_inner_tx.send(bytes::Bytes::from(ether_packet.packet().to_vec())) {
            Ok(_) => {
                crate::util::debug("send lsr packet success.");
            }
            Err(e) => {
                crate::util::error(&format!("send lsr packet failed:{}", e));
            }
        }
        tokio::time::sleep(interval).await;
    }
}

pub struct Handle {
    pub send_packet: Option<tokio::task::JoinHandle<()>>,
    pub recv_packet: Option<tokio::task::JoinHandle<()>>,
    pub hello_timer: Option<tokio::task::JoinHandle<()>>,
    pub wait_timer: Option<tokio::task::JoinHandle<()>>,
    #[allow(unused)]
    pub status_machine: Option<tokio::task::JoinHandle<()>>,
    pub dd_send: Option<tokio::task::JoinHandle<()>>,
    pub lsr_send: Option<tokio::task::JoinHandle<()>>,
    pub lsu_send: Option<tokio::task::JoinHandle<()>>,
    pub lsack_send: Option<tokio::task::JoinHandle<()>>,
}

impl Handle {
    pub fn new(addr: net::Ipv4Addr) -> Self {
        Self {
            send_packet: None,
            recv_packet: None,
            hello_timer: None,
            wait_timer: None,
            status_machine: Some(tokio::spawn(super::status::changed(addr))),
            dd_send: None,
            lsr_send: None,
            lsu_send: None,
            lsack_send: None,
        }
    }
    pub async fn when_interface_up(&mut self, iaddr: net::Ipv4Addr) -> super::status::Status {
        let raw_interface = super::get_raw_interface(iaddr).await;
        let locked_raw_interface = raw_interface.read().await;
        let (packet_tx, packet_rx) =
            match datalink::channel(&locked_raw_interface, default::Default::default()) {
                Ok(Ethernet(tx, rx)) => (tx, rx),
                _ => {
                    crate::util::error("create channel failed.");
                    return status::Status::Down;
                }
            };
        let interface_map = super::INTERFACE_MAP.read().await;
        let interface = interface_map.get(&iaddr).unwrap();
        let hello_interval = interface.hello_interval;
        let network_type = interface.network_type;
        let router_dead_interval = interface.router_dead_interval;
        drop(interface_map);
        self.send_packet = Some(tokio::spawn(send_packet(packet_tx, iaddr)));
        self.recv_packet = Some(tokio::spawn(recv_packet(packet_rx, iaddr)));
        self.hello_timer = Some(tokio::spawn(hello_timer(iaddr, hello_interval)));
        match network_type {
            super::NetworkType::Broadcast | super::NetworkType::NBMA => {
                self.wait_timer = Some(tokio::spawn(wait_timer(iaddr, router_dead_interval)));
                super::status::Status::Waiting
            }
            _ => super::status::Status::PointToPoint,
        }
    }

    pub async fn when_interface_down(&mut self, _iaddr: net::Ipv4Addr) {
        if let Some(send_packet) = self.send_packet.take() {
            send_packet.abort();
        }
        if let Some(recv_packet) = self.recv_packet.take() {
            recv_packet.abort();
        }
        if let Some(hello_timer) = self.hello_timer.take() {
            hello_timer.abort();
        }
        if let Some(wait_timer) = self.wait_timer.take() {
            wait_timer.abort();
        }
        if let Some(status_machine) = self.status_machine.take() {
            status_machine.abort();
        }
        if let Some(dd_send) = self.dd_send.take() {
            dd_send.abort();
        }
        if let Some(lsr_send) = self.lsr_send.take() {
            lsr_send.abort();
        }
        if let Some(lsu_send) = self.lsu_send.take() {
            lsu_send.abort();
        }
    }
}

pub async fn send_packet(mut packet_tx: Box<dyn DataLinkSender>, iaddr: net::Ipv4Addr) {
    let mut packet_inner_rx = super::trans::get_packet_inner_rx(iaddr).await;
    loop {
        match packet_inner_rx.recv().await {
            Ok(bytes) => {
                let ether_packet = match pnet::packet::ethernet::EthernetPacket::new(&bytes) {
                    Some(p) => p,
                    None => {
                        crate::util::error("receive the ethernet packet failed.");
                        continue;
                    }
                };
                match packet_tx.send_to(ether_packet.packet(), None) {
                    Some(res) => match res {
                        Ok(_) => {
                            crate::util::debug("send packet success.");
                        }
                        Err(e) => {
                            crate::util::error(&format!("send packet failed,{}", e));
                        }
                    },
                    None => {
                        crate::util::error("send packet failed.");
                    }
                }
            }
            Err(e) => {
                crate::util::error(&format!("receive inner packet failed,{}", e));
                continue;
            }
        }
    }
}

pub async fn wait_timer(iaddr: net::Ipv4Addr, router_dead_interval: u32) {
    let interval = tokio::time::Duration::from_secs(router_dead_interval as u64);
    tokio::time::sleep(interval).await;
    let event_senders = super::event::EVENT_SENDERS.read().await;
    let event_sender = match event_senders.get(&iaddr) {
        Some(event_sender) => event_sender,
        None => {
            crate::util::error("event sender not found.");
            return;
        }
    };
    match event_sender.send(super::event::Event::WaitTimer) {
        Ok(_) => {
            drop(event_senders);
            crate::util::debug("send wait timer event success.");
        }
        Err(_) => {
            drop(event_senders);
            crate::util::error("send wait timer event failed.");
        }
    }
}

pub async fn add(addr: net::Ipv4Addr, handle: Handle) {
    let mut handle_map = HANDLE_MAP.write().await;
    handle_map.insert(addr, Arc::new(RwLock::new(handle)));
}

pub async fn init(addrs: Vec<net::Ipv4Addr>) {
    for addr in addrs {
        let handle = Handle::new(addr);
        add(addr, handle).await;
    }
}

/// # is_ipv4_packet_valid
/// the function is used to check the ipv4 packet is valid or not
/// - packet : the ipv4 packet
/// - addr : the interface's ipv4 address
pub fn is_ipv4_packet_valid(_packet: &ipv4::Ipv4Packet) -> bool {
    true
}

/// the interface's packet rx.
pub async fn recv_packet(mut packet_rx: Box<dyn DataLinkReceiver>, iaddr: net::Ipv4Addr) -> () {
    loop {
        match packet_rx.next() {
            Ok(packet) => {
                let ether_packet = match ethernet::EthernetPacket::new(&packet) {
                    Some(p) => p,
                    None => {
                        crate::util::error("receive the ethernet packet failed.");
                        continue;
                    }
                };
                let ether_payload = ether_packet.payload();
                let src_mac = ether_packet.get_source();
                let ipv4_packet = match ipv4::Ipv4Packet::new(&ether_payload) {
                    Some(p) => p,
                    None => {
                        crate::util::error("receive the  packet failed.");
                        continue;
                    }
                };
                if !is_ipv4_packet_valid(&ipv4_packet) {
                    crate::util::error("invalid ipv4 packet.");
                    continue;
                }
                match ipv4_packet.get_next_level_protocol() {
                    ip::IpNextHeaderProtocol(OSPF_IP_PROTOCOL) => {
                        crate::util::debug("received ospf udp packet.");
                        match crate::packet::OspfPacket::try_from_ipv4_packet(&ipv4_packet) {
                            Ok(ospf_packet) => {
                                crate::packet::OspfPacket::received(
                                    ipv4_packet,
                                    ospf_packet,
                                    iaddr,
                                    src_mac,
                                )
                                .await;
                            }
                            Err(e) => {
                                crate::util::debug(&format!("{},ignored.", e));
                            }
                        }
                    }
                    _ => {
                        crate::util::debug(
                            "received non-ospf ip packet,forwarding or just received.",
                        );
                        rtable::forward_packet(iaddr, ipv4_packet).await;
                        continue;
                    }
                }
            }
            Err(_) => {
                crate::util::error("receive the udp packet failed.");
            }
        }
    }
}

pub async fn start_dd_send(
    iaddr: net::Ipv4Addr,
    naddr: net::Ipv4Addr,
    n_master: bool,
    init: bool,
    more: bool,
    lsa_headers: Option<Vec<lsa::Header>>,
) {
    let g_handles = HANDLE_MAP.read().await;
    let int_handles = g_handles.get(&iaddr).unwrap();
    let mut locked_in_handles = int_handles.write().await;
    if let Some(dd_send) = &locked_in_handles.dd_send {
        dd_send.abort();
    }
    let dd_send = tokio::spawn(dd_send(iaddr, naddr, n_master, init, more, lsa_headers));
    locked_in_handles.dd_send = Some(dd_send);
}

pub async fn hello_timer(iaddr: net::Ipv4Addr, hello_interval: u16) -> () {
    crate::util::debug("hello timer started.");
    let mut packet_inner_tx = super::trans::get_packet_inner_tx(iaddr).await;
    let mut buffer: Vec<u8> = vec![0; IPV4_PACKET_MTU];
    let src_mac = super::get_mac(iaddr).await;
    let dst_mac = [0x01, 0x00, 0x5E, 0x00, 0x00, 0x05].into();
    let mut bf: Vec<u8> = vec![0; IPV4_PACKET_MTU + 100];
    let interval = time::Duration::from_secs(hello_interval as u64);
    loop {
        let hello_packet = crate::packet::hello::Hello::new(iaddr).await;
        let hello_ipv4_packet = loop {
            match hello_packet.build_ipv4_packet(&mut buffer, iaddr).await {
                Ok(hello_ipv4_packet) => {
                    break hello_ipv4_packet;
                }
                Err(e) => {
                    crate::util::error(&format!("build hello packet failed:{}", e));
                }
            }
        };

        let ether_packet =
            crate::packet::build_ether_packet(&mut bf, hello_ipv4_packet, src_mac, dst_mac).await;
        loop {
            match packet_inner_tx.send(bytes::Bytes::from(ether_packet.packet().to_vec())) {
                Ok(_) => {
                    crate::util::debug("send hello packet success.");
                    break;
                }
                Err(e) => {
                    crate::util::error(&format!("send hello packet failed:{}", e));
                }
            }
        }

        tokio::time::sleep(interval).await;
    }
}

pub async fn stop_dd_send(iaddr: net::Ipv4Addr) {
    let g_handles = HANDLE_MAP.read().await;
    let int_handles = g_handles.get(&iaddr).unwrap();
    let mut locked_in_handles = int_handles.write().await;
    if let Some(dd_send) = &locked_in_handles.dd_send {
        dd_send.abort();
        locked_in_handles.dd_send = None;
    }
}

// just send one packet
pub async fn dd_send(
    iaddr: net::Ipv4Addr,
    naddr: net::Ipv4Addr,
    n_master: bool,
    init: bool,
    more: bool,
    lsa_headers: Option<Vec<lsa::Header>>,
) {
    let rxmt_interval = super::get_rxmt_interval(iaddr).await;
    let interval = time::Duration::from_secs(rxmt_interval as u64);
    let packet_inner_tx = super::trans::get_packet_inner_tx(iaddr).await;
    let mut dd_flags = if n_master { 0 } else { FLAG_MS_BIT };
    if init {
        dd_flags |= FLAG_I_BIT;
    }
    if more {
        dd_flags |= FLAG_M_BIT;
    }
    let dd_options = 0x42;
    let seqno = get_ddseqno(iaddr, naddr).await;
    let mut buffer = vec![0; IPV4_PACKET_MTU];
    let mut bf = vec![0; IPV4_PACKET_MTU + 100];
    let src_mac = super::get_mac(iaddr).await;
    let dst_mac = neighbor::get_mac(iaddr, naddr).await;
    loop {
        let dd = DD::new(
            iaddr,
            naddr,
            dd_options,
            dd_flags,
            seqno,
            lsa_headers.clone(),
        )
        .await;
        let ippacket = dd.build_ipv4_packet(&mut buffer, iaddr, naddr).unwrap();
        let ethet_packet =
            crate::packet::build_ether_packet(&mut bf, ippacket, src_mac, dst_mac).await;
        match packet_inner_tx.send(bytes::Bytes::from(ethet_packet.packet().to_vec())) {
            Ok(_) => {
                crate::util::debug("send dd packet success.");
                break;
            }
            Err(e) => {
                crate::util::error(&format!("send dd packet failed:{}", e));
            }
        }
        save_last_dd(iaddr, naddr, dd.clone()).await;
        time::sleep(interval).await;
    }
}
