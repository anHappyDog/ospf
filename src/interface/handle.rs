use crate::neighbor::get_ddseqno;
use crate::packet::dd::FLAG_MS_BIT;
use crate::packet::lsack::Lsack;
use crate::packet::lsu::Lsu;
use crate::{area, lsa, OPTION_E};
use pnet::datalink::Channel::Ethernet;
use pnet::datalink::{DataLinkReceiver, DataLinkSender};
use pnet::packet::ethernet::EtherTypes;
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
use std::{collections::HashMap, net, sync::Arc};
use tokio::task::JoinHandle;
use tokio::{sync::RwLock, time};

use crate::{neighbor, packet::dd::DD, IPV4_PACKET_MTU, OSPF_IP_PROTOCOL};

use super::status;

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
    let packet_sender = super::trans::PACKET_SENDER.clone();
    let retrans_list = neighbor::get_trans_list(iaddr, naddr).await;
    let mut buffer: Vec<u8> = vec![0; crate::IPV4_PACKET_MTU];
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
        match packet_sender.send(bytes::Bytes::from(ip_packet.packet().to_vec())) {
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

pub async fn create_router_lsa(iaddr: net::Ipv4Addr) {}

pub async fn create_network_lsa(iaddr: net::Ipv4Addr) {}

pub async fn create_summary_lsa(iaddr: net::Ipv4Addr) {}

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
    let packet_sender = super::trans::PACKET_SENDER.clone();
    match packet_sender.send(bytes::Bytes::from(ip_packet.packet().to_vec())) {
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
    let packet_sender = super::trans::PACKET_SENDER.clone();
    loop {
        let lsr_packet = crate::packet::lsr::Lsr::new(iaddr, lsr_list.clone()).await;
        let mut buffer = vec![0; IPV4_PACKET_MTU];
        let ippacket = lsr_packet
            .build_ipv4_packet(&mut buffer, iaddr, naddr)
            .await
            .unwrap();
        match packet_sender.send(bytes::Bytes::from(ippacket.packet().to_vec())) {
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
        let mut tr_config = Config::default();
        tr_config.channel_type = datalink::ChannelType::Layer3(EtherTypes::Ipv4.0);
        let (packet_tx, packet_rx) = match datalink::channel(&locked_raw_interface, tr_config) {
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

    pub async fn when_interface_down(&mut self, iaddr: net::Ipv4Addr) {
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

pub async fn global_packet_send() {
    let (mut packet_tx, _) = transport::transport_channel(
        1024,
        transport::TransportChannelType::Layer3(IpNextHeaderProtocols::Ipv4),
    )
    .expect("create packet sender failed.");
    let mut packet_inner_rx = super::trans::PACKET_SENDER.clone().subscribe();
    loop {
        match packet_inner_rx.recv().await {
            Ok(bytes) => match Ipv4Packet::new(&bytes) {
                Some(packet) => {
                    let destination = packet.get_destination();
                    match packet_tx.send_to(packet, destination.into()) {
                        Ok(_) => {
                            crate::util::debug("send ip packet success.");
                        }
                        _ => {
                            crate::util::debug("send ip packet failed.");
                        }
                    }
                }
                None => {
                    crate::util::error("received non-ip inner packet.");
                }
            },
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
    let mut g_packet_send = PACKET_SEND.write().await;
    *g_packet_send = Some(tokio::spawn(global_packet_send()));
    for addr in addrs {
        let handle = Handle::new(addr);
        add(addr, handle).await;
    }
}

/// # is_ipv4_packet_valid
/// the function is used to check the ipv4 packet is valid or not
/// - packet : the ipv4 packet
/// - addr : the interface's ipv4 address
pub fn is_ipv4_packet_valid(packet: &ipv4::Ipv4Packet) -> bool {
    true
}

/// the interface's packet rx.
pub async fn recv_packet(mut packet_rx: Box<dyn DataLinkReceiver>, iaddr: net::Ipv4Addr) -> () {
    loop {
        match packet_rx.next() {
            Ok(packet) => {
                let ipv4_packet = match ipv4::Ipv4Packet::new(&packet) {
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
                                )
                                .await;
                            }
                            Err(e) => {
                                crate::util::debug(&format!("{},ignored.", e));
                            }
                        }
                    }
                    _ => {
                        // crate::util::debug(
                        //     "received non-ospf ip packet,forwarding or just received.",
                        // );
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
    lsa_headers: Option<Vec<lsa::Header>>,
) {
    let g_handles = HANDLE_MAP.read().await;
    let int_handles = g_handles.get(&iaddr).unwrap();
    let mut locked_in_handles = int_handles.write().await;
    if let Some(dd_send) = &locked_in_handles.dd_send {
        dd_send.abort();
    }
    let dd_send = tokio::spawn(dd_send(iaddr, naddr, n_master, lsa_headers));
    locked_in_handles.dd_send = Some(dd_send);
}

pub async fn wait_dd(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) -> Option<DD> {
    let g_trans = super::trans::TRANSMISSIONS.read().await;
    let trans = g_trans.get(&iaddr).unwrap();
    let mut inner_dd_rx = trans.inner_dd_tx.subscribe();
    match inner_dd_rx.recv().await {
        Ok(dd) => {
            crate::util::debug("received dd packet,notified.");
            return Some(dd);
        }
        Err(e) => {
            crate::util::error(&format!("receive dd packet failed:{}", e));
            return None;
        }
    }
}

pub async fn hello_timer(iaddr: net::Ipv4Addr, hello_interval: u16) -> () {
    crate::util::debug("hello timer started.");
    let inner_packet_tx = super::trans::PACKET_SENDER.clone();
    let mut buffer: Vec<u8> = vec![0; IPV4_PACKET_MTU - 100];
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
        loop {
            match inner_packet_tx.send(bytes::Bytes::from(hello_ipv4_packet.packet().to_vec())) {
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
    lsa_headers: Option<Vec<lsa::Header>>,
) {
    let rxmt_interval = super::get_rxmt_interval(iaddr).await;
    let interval = time::Duration::from_secs(rxmt_interval as u64);
    let packet_sender = super::trans::PACKET_SENDER.clone();
    let dd_flags = if n_master { 0 } else { FLAG_MS_BIT };
    let dd_options = OPTION_E;
    let seqno = get_ddseqno(iaddr, naddr).await;
    let mut buffer = vec![0; IPV4_PACKET_MTU];
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
        match packet_sender.send(bytes::Bytes::from(ippacket.packet().to_vec())) {
            Ok(_) => {
                crate::util::debug("send dd packet success.");
                break;
            }
            Err(e) => {
                crate::util::error(&format!("send dd packet failed:{}", e));
            }
        }
        time::sleep(interval).await;
    }
}
