use std::{collections::HashMap, net, sync::Arc};

use pnet::{
    packet::{
        ip::{
            IpNextHeaderProtocol,
            IpNextHeaderProtocols::{Tcp, Udp},
        },
        ipv4,
        tcp::Tcp,
        Packet,
    },
    transport,
};
use tokio::sync::{broadcast, RwLock};

use crate::{
    packet::{
        self, dd::DD_TYPE, hello::HELLO_TYPE, lsack::LSACK_TYPE, lsr::LSR_TYPE, lsu::LSU_TYPE,
        OspfPacket,
    },
    util, OSPF_IP_PROTOCOL,
};

use super::{trans, NetworkType};

/// # Handler
/// the data structure is used to store the handler for the interface
/// the handler contains the JoinHandle for send_tcp,send_udp,recv_tcp,recv_udp
/// wait_timer and hello_timer
pub struct Handler {
    pub send_tcp: Option<tokio::task::JoinHandle<()>>,
    pub send_udp: Option<tokio::task::JoinHandle<()>>,
    pub recv_tcp: Option<tokio::task::JoinHandle<()>>,
    pub recv_udp: Option<tokio::task::JoinHandle<()>>,
    pub wait_timer: Option<tokio::task::JoinHandle<()>>,
    pub hello_timer: Option<tokio::task::JoinHandle<()>>,
}

pub async fn init_when_interface_up(
    ipv4_addr: net::Ipv4Addr,
    interface_name : String,
    network_type: NetworkType,
    router_priority: u8,
) {
    let (tcp_tx, tcp_rx) =
        transport::transport_channel(1024, transport::TransportChannelType::Layer3(Tcp)).unwrap();
    let (udp_tx, udp_rx) =
        transport::transport_channel(1024, transport::TransportChannelType::Layer3(Udp)).unwrap();
    let handlers = HANDLERS.read().await;
    let transmissions = super::trans::TRANSMISSIONS.write().await;
    let tcp_inner_rx = transmissions
        .get(&ipv4_addr)
        .unwrap()
        .write()
        .await
        .inner_tcp_tx
        .subscribe();
    let udp_inner_rx = transmissions
        .get(&ipv4_addr)
        .unwrap()
        .write()
        .await
        .inner_udp_tx
        .subscribe();
    let udp_inner_tx = transmissions
        .get(&ipv4_addr)
        .unwrap()
        .write()
        .await
        .inner_udp_tx
        .clone();
    let mut handler = handlers.get(&ipv4_addr).unwrap().write().await;
    handler.send_tcp = Some(tokio::spawn(send_tcp(tcp_tx, tcp_inner_rx)));
    handler.send_udp = Some(tokio::spawn(send_udp(udp_tx, udp_inner_rx)));
    handler.recv_tcp = Some(tokio::spawn(recv_tcp(tcp_rx)));
    handler.recv_udp = Some(tokio::spawn(recv_udp(udp_rx, udp_inner_tx,interface_name.clone())));
    handler.hello_timer = Some(tokio::spawn(hello_timer()));
    match network_type {
        NetworkType::Broadcast | NetworkType::NBMA => {
            if router_priority != 0 {
                handler.wait_timer = Some(tokio::spawn(wait_timer()));
            }
        }
        _ => {}
    }
}

pub async fn when_interface_down(interface_name: String) {}

lazy_static::lazy_static! {
    /// # HANDLERS
    /// the data structure is used to store the handlers for the interface
    /// use the ipv4 address of the interface to index the handler
    pub static ref HANDLERS : Arc<RwLock<HashMap<net::Ipv4Addr, Arc<RwLock<Handler>>>>> = Arc::new(RwLock::new(HashMap::new()));
}

/// # send_tcp
/// the function is used to create the future handle for send tcp ipv4 packet
/// - tcp_tx : the sender for the tcp handler
/// - tcp_inner_rx : the receiver for inner interface or other interfaces, to forward the ipv4 packet
pub async fn send_tcp(
    mut tcp_tx: transport::TransportSender,
    mut tcp_inner_rx: broadcast::Receiver<bytes::Bytes>,
) {
    loop {
        match tcp_inner_rx.recv().await {
            Ok(packet) => {
                let ipv4_packet = ipv4::Ipv4Packet::new(&packet).unwrap();
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
) {
    loop {
        match udp_inner_rx.recv().await {
            Ok(packet) => {
                let ipv4_packet = ipv4::Ipv4Packet::new(&packet).unwrap();
                let destination = ipv4_packet.get_destination();
                match udp_tx.send_to(ipv4_packet, destination.into()) {
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

pub fn try_get_ospf_packet(
    ipv4_packet: &ipv4::Ipv4Packet,
    interface_name: String,
) -> Result<OspfPacket, &'static str> {
    match ipv4_packet.get_next_level_protocol() {
        IpNextHeaderProtocol(OSPF_IP_PROTOCOL) => {
            let payload = ipv4_packet.payload();
            match packet::OspfHeader::try_from_be_bytes(payload) {
                Some(ospf_header) => match ospf_header.packet_type {
                    HELLO_TYPE => {
                        let hello_packet = packet::hello::Hello::try_from_be_bytes(payload);
                        match hello_packet {
                            Some(hello_packet) => {
                                util::debug("ospf hello packet received.");
                                Ok(OspfPacket::Hello(hello_packet))
                            }
                            None => Err("invalid hello packet,ignored."),
                        }
                    }
                    DD_TYPE => {
                        let dd_packet = packet::dd::DD::try_from_be_bytes(payload);
                        match dd_packet {
                            Some(dd_packet) => {
                                util::debug("ospf dd packet received.");
                                Ok(OspfPacket::DD(dd_packet))
                            }
                            None => Err("invalid dd packet,ignored."),
                        }
                    }
                    LSR_TYPE => {
                        let lsr_packet = packet::lsr::Lsr::try_from_be_bytes(payload);
                        match lsr_packet {
                            Some(lsr_packet) => {
                                util::debug("ospf lsr packet received.");
                                Ok(OspfPacket::LSR(lsr_packet))
                            }
                            None => Err("invalid lsr packet,ignored."),
                        }
                    }
                    LSU_TYPE => {
                        let lsu_packet = packet::lsu::Lsu::try_from_be_bytes(payload);
                        match lsu_packet {
                            Some(lsu_packet) => {
                                util::debug("ospf lsu packet received.");
                                Ok(OspfPacket::LSU(lsu_packet))
                            }
                            None => Err("invalid lsu packet,ignored."),
                        }
                    }
                    LSACK_TYPE => {
                        let lsack_packet = packet::lsack::Lsack::try_from_be_bytes(payload);
                        match lsack_packet {
                            Some(lsack_packet) => {
                                util::debug("ospf lsack packet received.");
                                Ok(OspfPacket::LSACK(lsack_packet))
                            }
                            None => Err("invalid lsack packet,ignored."),
                        }
                    }
                    _ => Err("invalid ospf packet type,ignored."),
                },
                None => Err("invalid ospf packet,ignored."),
            }
        }
        _ => Err("non-ospf packet received."),
    }
}

/// # recv_udp
/// the function is used to create the future handle for recv udp ipv4 packet
/// - udp_rx : the receiver for the udp handler
/// - udp_inner_tx : the sender for inner interface or other interfaces, to forward the ipv4 packet
/// - the ipv4 address of the interface
/// the function will receive the ipv4 packet from the udp handler and forward the packet to the inner interface or other interfaces
/// the function will loop until the ipv4 packet is received
pub async fn recv_udp(
    mut udp_rx: transport::TransportReceiver,
    mut udp_inner_tx: broadcast::Sender<bytes::Bytes>,
    interface_name: String,
) {
    let mut ipv4_packet_iter = transport::ipv4_packet_iter(&mut udp_rx);
    loop {
        match ipv4_packet_iter.next() {
            Ok((ipv4_packet, ip)) => {
                if !is_ipv4_packet_valid(&ipv4_packet) {
                    util::error("invalid ipv4 packet.");
                    continue;
                }
                match try_get_ospf_packet(&ipv4_packet, interface_name.clone()) {
                    Ok(ospf_packet) => {}
                    Err(_) => {
                        continue;
                    }
                }
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

/// # recv_tcp
/// the function is used to create the future handle for recv tcp ipv4 packet
/// - tcp_rx : the receiver for the tcp handler
/// the function will receive the ipv4 packet from the tcp handler
/// the function will loop until the ipv4 packet is received
pub async fn recv_tcp(mut tcp_rx: transport::TransportReceiver) {
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

pub async fn wait_timer() {}

pub async fn hello_timer() {}

pub async fn init_handlers(ipv4_addrs: Vec<net::Ipv4Addr>) {
    let mut handlers = HANDLERS.write().await;
    ipv4_addrs.iter().for_each(|ipv4_addr| {
        handlers.insert(
            ipv4_addr.clone(),
            Arc::new(RwLock::new(Handler {
                send_tcp: None,
                send_udp: None,
                recv_tcp: None,
                recv_udp: None,
                wait_timer: None,
                hello_timer: None,
            })),
        );
    })
}
