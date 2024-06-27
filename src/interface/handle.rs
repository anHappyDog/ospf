use pnet::datalink::Channel::Ethernet;
use pnet::datalink::{DataLinkReceiver, DataLinkSender};
use pnet::packet::ethernet::EtherTypes;
use pnet::{
    datalink::{self, Config},
    packet::{
        self,
        ip::{
            self,
            IpNextHeaderProtocols::{Tcp, Udp},
        },
        ipv4, Packet,
    },
    transport,
};
use socket2::{Domain, Protocol, Type};
use std::{collections::HashMap, net, sync::Arc};
use tokio::{
    sync::{broadcast, RwLock},
    time,
};

use crate::{
    neighbor,
    packet::{dd::DD, hello::HELLO_TYPE},
    IPV4_PACKET_MTU, OSPF_IP_PROTOCOL,
};

use super::status;

// ONLY USE IN THE INNER INTERFACE
// KEY IS THE INTERFACE 'S IPV4 ADDR
lazy_static::lazy_static! {
    pub static ref HANDLE_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Handle>>>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub struct Handle {
    pub send_packet: Option<tokio::task::JoinHandle<()>>,
    pub recv_packet: Option<tokio::task::JoinHandle<()>>,
    pub hello_timer: Option<tokio::task::JoinHandle<()>>,
    pub wait_timer: Option<tokio::task::JoinHandle<()>>,
    #[allow(unused)]
    pub status_machine: Option<tokio::task::JoinHandle<()>>,
    pub dd_negoation: Option<tokio::task::JoinHandle<()>>,
    pub dd_master_send: Option<tokio::task::JoinHandle<()>>,
    pub dd_slave_send: Option<tokio::task::JoinHandle<()>>,
    pub lsr_send: Option<tokio::task::JoinHandle<()>>,
    pub lsu_send: Option<tokio::task::JoinHandle<()>>,
}

impl Handle {
    pub fn new(addr: net::Ipv4Addr) -> Self {
        Self {
            send_packet: None,
            recv_packet: None,
            hello_timer: None,
            wait_timer: None,
            status_machine: Some(tokio::spawn(super::status::changed(addr))),
            dd_negoation: None,
            dd_slave_send: None,
            dd_master_send: None,
            lsr_send: None,
            lsu_send: None,
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
        let g_trans = super::trans::TRANSMISSIONS.read().await;
        let trans = g_trans.get(&iaddr).unwrap();
        self.send_packet = Some(tokio::spawn(send_packet(
            packet_tx,
            trans.inner_packet_tx.subscribe(),
        )));
        self.recv_packet = Some(tokio::spawn(recv_packet(packet_rx, iaddr)));

        self.hello_timer = Some(tokio::spawn(hello_timer(
            iaddr,
            trans.inner_packet_tx.clone(),
            hello_interval,
        )));
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
        if let Some(dd_negoation) = self.dd_negoation.take() {
            dd_negoation.abort();
        }
        if let Some(dd_slave_send) = self.dd_slave_send.take() {
            dd_slave_send.abort();
        }
        if let Some(dd_master_send) = self.dd_master_send.take() {
            dd_master_send.abort();
        }
        if let Some(lsr_send) = self.lsr_send.take() {
            lsr_send.abort();
        }
        if let Some(lsu_send) = self.lsu_send.take() {
            lsu_send.abort();
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
                        crate::util::debug(
                            "received non-ospf udp packet,forwarding or just received.",
                        );
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

/// # send_packet
/// the function is used to create the future handle for send udp ipv4 packet
/// - packet_tx : the sender for the udp handler
/// - udp_inner_rx : the receiver for inner interface or other interfaces, to forward the ipv4 packet
pub async fn send_packet(
    mut packet_tx: Box<dyn DataLinkSender>,
    mut inner_packet_rx: broadcast::Receiver<bytes::Bytes>,
) -> () {
    let socket = socket2::Socket::new(Domain::IPV4,Type::RAW,Some(Protocol::UDP)).unwrap();
    
    loop {
        match inner_packet_rx.recv().await {
            Ok(packet) => match packet_tx.send_to(&packet, None) {
                Some(result) => match result {
                    Ok(_) => {
                        let ip_packet = match ipv4::Ipv4Packet::new(&packet) {
                            Some(ip_packet) => ip_packet,
                            None => {
                                crate::util::error("receive the inner packet failed.");
                                continue;
                            }
                        };
                        crate::util::debug(&format!("packet source is {},destionation is {}",ip_packet.get_source(),ip_packet.get_destination()));    
                        crate::util::debug(&format!("packet is {:?}",ip_packet));
                        crate::util::debug("send packet success.");
                    }
                    Err(e) => {
                        crate::util::error(&format!("send packet failed:{}", e));
                    }
                },
                None => {
                    crate::util::error("send packet failed.");
                }
            },
            Err(_) => {
                crate::util::error("receive the inner packet failed.");
                continue;
            }
        }
    }
}

pub async fn hello_timer(
    iaddr: net::Ipv4Addr,
    udp_inner_tx: broadcast::Sender<bytes::Bytes>,
    hello_interval: u16,
) -> () {
    crate::util::debug("hello timer started.");
    let mut buffer: Vec<u8> = vec![0; IPV4_PACKET_MTU];
    let interval = time::Duration::from_secs(hello_interval as u64);
    loop {
        tokio::time::sleep(interval).await;
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
            match udp_inner_tx.send(bytes::Bytes::from(hello_ipv4_packet.packet().to_vec())) {
                Ok(_) => {
                    crate::util::debug("send hello packet success.");
                    break;
                }
                Err(e) => {
                    crate::util::error(&format!("send hello packet failed:{}", e));
                }
            }
        }
    }
}

pub async fn start_dd_negoation(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) {
    let handles = HANDLE_MAP.read().await;
    let handle = handles.get(&iaddr).unwrap();
    let mut int_handle = handle.write().await;
    let g_trans = super::trans::TRANSMISSIONS.read().await;
    let trans = g_trans.get(&iaddr).unwrap();
    int_handle.dd_negoation = Some(tokio::spawn(dd_negoation(
        trans.inner_packet_tx.clone(),
        iaddr,
        naddr,
    )));
}

pub async fn dd_negoation(
    udp_inner_tx: broadcast::Sender<bytes::Bytes>,
    iaddr: net::Ipv4Addr,
    naddr: net::Ipv4Addr,
) {
    let interface_map = super::INTERFACE_MAP.read().await;
    let interface = interface_map.get(&iaddr).unwrap();
    let rxmt_interval = interface.rxmt_interval;
    let duration = time::Duration::from_secs(rxmt_interval as u64);
    let dd_options = 0;
    let dd_flags = 0;
    let ddseq = neighbor::get_ddseqno(iaddr, naddr).await;

    loop {
        tokio::time::sleep(duration).await;
        let dd_packet = DD::new(iaddr, naddr, dd_options, dd_flags, ddseq).await;
        let mut buffer: Vec<u8> = vec![0; IPV4_PACKET_MTU];
        let ip_packet = match dd_packet.build_ipv4_packet(&mut buffer, iaddr, naddr) {
            Some(ip_packet) => ip_packet,
            None => {
                crate::util::error(&format!("build dd packet failed."));
                continue;
            }
        };
        match udp_inner_tx.send(bytes::Bytes::from(ip_packet.packet().to_vec())) {
            Ok(_) => {
                crate::util::debug("send dd packet success.");
            }
            Err(e) => {
                crate::util::error(&format!("send dd packet failed:{}", e));
            }
        }
    }
}
