use std::{collections::HashMap, net, sync::Arc};

use pnet::{
    packet::{
        self,
        ip::{self, IpNextHeaderProtocols::Udp},
        ipv4, Packet,
    },
    transport,
};
use tokio::{
    sync::{broadcast, RwLock},
    time,
};

use crate::{
    neighbor,
    packet::{dd::DD, hello::HELLO_TYPE},
    IPV4_PACKET_MTU, OSPF_IP_PROTOCOL,
};

// ONLY USE IN THE INNER INTERFACE
// KEY IS THE INTERFACE 'S IPV4 ADDR
lazy_static::lazy_static! {
    pub static ref HANDLE_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Handle>>>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub struct Handle {
    pub send_tcp: Option<tokio::task::JoinHandle<()>>,
    pub send_udp: Option<tokio::task::JoinHandle<()>>,
    pub recv_tcp: Option<tokio::task::JoinHandle<()>>,
    pub recv_udp: Option<tokio::task::JoinHandle<()>>,
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
            send_tcp: None,
            send_udp: None,
            recv_tcp: None,
            recv_udp: None,
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
        let (udp_tx, udp_rx) =
            transport::transport_channel(1024, transport::TransportChannelType::Layer3(Udp))
                .unwrap();
        let (tcp_tx, tcp_rx) =
            transport::transport_channel(1024, transport::TransportChannelType::Layer3(Udp))
                .unwrap();
        let interface_map = super::INTERFACE_MAP.read().await;
        let interface = interface_map.get(&iaddr).unwrap();
        let hello_interval = interface.hello_interval;
        let network_type = interface.network_type;
        let router_dead_interval = interface.router_dead_interval;
        drop(interface_map);
        let g_trans = super::trans::TRANSMISSIONS.read().await;
        let trans = g_trans.get(&iaddr).unwrap();
        self.send_tcp = Some(tokio::spawn(send_tcp(
            tcp_tx,
            trans.inner_tcp_tx.subscribe(),
        )));
        self.send_udp = Some(tokio::spawn(send_udp(
            udp_tx,
            trans.inner_udp_tx.subscribe(),
        )));
        self.recv_tcp = Some(tokio::spawn(recv_tcp(tcp_rx)));
        self.recv_udp = Some(tokio::spawn(recv_udp(udp_rx, iaddr)));
        self.hello_timer = Some(tokio::spawn(hello_timer(
            iaddr,
            trans.inner_udp_tx.clone(),
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
        if let Some(send_tcp) = self.send_tcp.take() {
            send_tcp.abort();
        }
        if let Some(send_udp) = self.send_udp.take() {
            send_udp.abort();
        }
        if let Some(recv_tcp) = self.recv_tcp.take() {
            recv_tcp.abort();
        }
        if let Some(recv_udp) = self.recv_udp.take() {
            recv_udp.abort();
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

/// # recv_tcp
/// the function is used to create the future handle for recv tcp ipv4 packet
/// - tcp_rx : the receiver for the tcp handler
/// the function will receive the ipv4 packet from the tcp handler
/// the function will loop until the ipv4 packet is received
pub async fn recv_tcp(mut tcp_rx: transport::TransportReceiver) -> () {
    let mut ipv4_packet_iter = transport::ipv4_packet_iter(&mut tcp_rx);
    loop {
        match ipv4_packet_iter.next() {
            Ok((ipv4_packet, ip)) => {
                // if !is_ipv4_packet_valid(&ipv4_packet, ) {
                //     util::error("invalid ipv4 packet.");
                //     continue;
                // }
            }
            Err(_) => {
                continue;
            }
        }
    }
}

/// # is_ipv4_packet_valid
/// the function is used to check the ipv4 packet is valid or not
/// - packet : the ipv4 packet
/// - addr : the interface's ipv4 address
pub fn is_ipv4_packet_valid(packet: &ipv4::Ipv4Packet) -> bool {
    true
}

/// # recv_udp
/// the function is used to create the future handle for recv udp ipv4 packet
/// - udp_rx : the receiver for the udp handler
/// - udp_inner_tx : the sender for inner interface or other interfaces, to forward the ipv4 packet
/// - the ipv4 address of the interface
/// the function will receive the ipv4 packet from the udp handler and forward the packet to the inner interface or other interfaces
/// the function will loop until the ipv4 packet is received
pub async fn recv_udp(mut udp_rx: transport::TransportReceiver, ipv4_addr: net::Ipv4Addr) -> () {
    let mut ipv4_packet_iter = transport::ipv4_packet_iter(&mut udp_rx);
    loop {
        match ipv4_packet_iter.next() {
            Ok((ipv4_packet, ip)) => {
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
                                    ipv4_addr,
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

/// # send_tcp
/// the function is used to create the future handle for send tcp ipv4 packet
/// - tcp_tx : the sender for the tcp handler
/// - tcp_inner_rx : the receiver for inner interface or other interfaces, to forward the ipv4 packet
pub async fn send_tcp(
    mut tcp_tx: transport::TransportSender,
    mut tcp_inner_rx: broadcast::Receiver<bytes::Bytes>,
) -> () {
    loop {
        match tcp_inner_rx.recv().await {
            Ok(packet) => {
                let ipv4_packet = match ipv4::Ipv4Packet::new(&packet) {
                    Some(ipv4_packet) => ipv4_packet,
                    None => {
                        crate::util::error("receive the tcp inner ip packet failed.");
                        continue;
                    }
                };
                let destination = ipv4_packet.get_destination();
                match tcp_tx.send_to(ipv4_packet, destination.into()) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
            Err(_) => {
                continue;
            }
        }
    }
}

/// # send_udp
/// the function is used to create the future handle for send udp ipv4 packet
/// - udp_tx : the sender for the udp handler
/// - udp_inner_rx : the receiver for inner interface or other interfaces, to forward the ipv4 packet
pub async fn send_udp(
    mut udp_tx: transport::TransportSender,
    mut udp_inner_rx: broadcast::Receiver<bytes::Bytes>,
) -> () {
    loop {
        match udp_inner_rx.recv().await {
            Ok(packet) => {
                let ipv4_packet = match ipv4::Ipv4Packet::new(&packet) {
                    Some(ipv4_packet) => ipv4_packet,
                    None => {
                        crate::util::error("receive the udp inner ip packet failed.");
                        continue;
                    }
                };
                let destination = ipv4_packet.get_destination();
                match udp_tx.send_to(ipv4_packet, destination.into()) {
                    Ok(_) => {
                        crate::util::debug("send udp packet success.");
                    }
                    Err(e) => {
                        crate::util::error(&format!("send udp packet failed:{}", e));
                    }
                }
            }
            Err(_) => {
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
        trans.inner_udp_tx.clone(),
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
