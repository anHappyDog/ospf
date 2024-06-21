use std::{
    collections::HashMap,
    net,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use pnet::{
    packet::{
        ip::IpNextHeaderProtocol,
        ipv4::{self, Ipv4Packet},
        udp, Packet,
    },
    transport::{self, ipv4_packet_iter},
};
use tokio::{sync::broadcast, time};

use crate::{
    error,
    lsa::router,
    neighbor,
    packet::{self, is_ospf_packet_valid, new_ip_packet, OspfPacket},
    OSPF_IP_PROTOCOL_NUMBER, OSPF_VERSION_2,
};

pub enum OpsfHandleType {
    SendTcpPacket,
    RecvTcpPacket,
    SendUdpPacket,
    RecvUdpPacket,
    CreateHelloPacket,
    CreateDataDescriptionPacket,
}

pub struct OspfHandle<T> {
    pub raw_handle: JoinHandle<T>,
    pub handle_type: OpsfHandleType,
}

impl<T> OspfHandle<T> {
    pub fn new(raw_handle: JoinHandle<T>, handle_type: OpsfHandleType) -> Self {
        Self {
            raw_handle,
            handle_type,
        }
    }
}

pub async fn send_tcp_packet_raw_handle(
    mut to_send_tcp_packet_rx: broadcast::Receiver<bytes::Bytes>,
    mut tcp_ip_packet_tx: transport::TransportSender,
) {
    loop {
        if let Ok(packet_bytes) = to_send_tcp_packet_rx.recv().await {
            if let Some(packet) = Ipv4Packet::new(&packet_bytes) {
                let destination = packet.get_destination();
                if let Ok(_) = tcp_ip_packet_tx.send_to(packet, net::IpAddr::V4(destination)) {
                    crate::debug("interface sending the tcp-sending packet success.");
                } else {
                    crate::error("interface sending the tcp-sending packet failed.");
                }
            } else {
                crate::error("interface parsing the tcp-sending packet failed.");
            }
        } else {
            crate::error("interface receiving the tcp-sending packet failed.");
        }
    }
}

pub async fn send_udp_packet_raw_handle(
    mut to_send_udp_packet_rx: broadcast::Receiver<bytes::Bytes>,
    mut udp_ip_packet_tx: transport::TransportSender,
) {
    loop {
        if let Ok(packet_bytes) = to_send_udp_packet_rx.recv().await {
            if let Some(packet) = Ipv4Packet::new(&packet_bytes) {
                let destination = packet.get_destination();
                if let Ok(_) = udp_ip_packet_tx.send_to(packet, net::IpAddr::V4(destination)) {
                    crate::debug("interface sending the udp-sending packet success.");
                } else {
                    crate::error("interface sending the udp-sending packet failed.");
                }
            } else {
                crate::error("interface parsing the udp-sending packet failed.");
            }
        } else {
            crate::error("interface receiving the udp-sending packet failed.");
        }
    }
}

pub async fn recv_tcp_packet_raw_handle(
    mut to_send_tcp_packet_tx: broadcast::Sender<bytes::Bytes>,
    mut tcp_ip_packet_rx: transport::TransportReceiver,
) {
    let mut ipv4_packet_iter = ipv4_packet_iter(&mut tcp_ip_packet_rx);
    loop {
        if let Ok((packet, _ip)) = ipv4_packet_iter.next() {
            if !packet::is_ip_packet_valid(&packet) {
                crate::error("interface received invalid ip packet.");
                continue;
            } else {
                crate::debug(&format!(
                    "interface received valid ip packet from tcp.[{:#?}]",
                    packet
                ));
            }
        } else {
            crate::error("interface recv ip packet failed.");
        }
    }
}
///
/// # recv_udp_packet_raw_handle
///this function is used to handle the received udp packet from the interface.
///it can receive udp and tcp packet, if received ospf packet, then handle it.
///otherwise, just forwarding it or receive it.
///
pub async fn recv_udp_packet_raw_handle(
    mut to_send_udp_packet_tx: broadcast::Sender<bytes::Bytes>,
    mut udp_ip_packet_rx: transport::TransportReceiver,
) {
    let mut ipv4_packet_iter = ipv4_packet_iter(&mut udp_ip_packet_rx);
    loop {
        if let Ok((packet, _ip)) = ipv4_packet_iter.next() {
            if !packet::is_ip_packet_valid(&packet) {
                crate::error("interface received invalid ip packet.");
                continue;
            }
            // handle the received ip packet.
            match packet.get_next_level_protocol() {
                IpNextHeaderProtocol(OSPF_IP_PROTOCOL_NUMBER) => {
                    crate::debug("interface received ospf udp packet.");
                    let neighbors = Arc::new(Mutex::new(HashMap::new()));
                    if let Ok(ospf_packet) =
                        packet::try_get_from_ipv4_packet(&packet, neighbors.clone())
                    {
                        crate::debug("interface received ospf udp packet and parse success.");
                        if !is_ospf_packet_valid(ospf_packet.as_ref()) {
                            crate::error("interface received ospf udp packet but checksum failed.");
                            continue;
                        }
                        match ospf_packet.get_type() {
                            packet::hello::HELLO_PACKET_TYPE => {
                                crate::debug(
                                    "interface received hello packet,try to update its neighbors",
                                );
                            }
                            packet::dd::DATA_DESCRIPTION_PACKET_TYPE => {
                                crate::debug("interface received dd packet.");
                            }
                            packet::lsr::LINK_STATE_REQUEST_PACKET_TYPE => {
                                crate::debug("interface received lsr packet.");
                            }
                            packet::lsack::LINK_STATE_ACKNOWLEDGEMENT_PACKET_TYPE => {
                                crate::debug("interface received lsack packet.");
                            }
                            packet::lsu::LINK_STATE_UPDATE_TYPE => {
                                crate::debug("interface received lsu packet.");
                            }
                            _ => {
                                crate::error("interface received unknown ospf packet.");
                            }
                        }
                    } else {
                        crate::error(
                            "interface received proto-num 89 udp packet but parse failed.",
                        );
                    }
                }
                _ => {
                    crate::debug("interface received proto-num non-89 udp packet.");
                }
            }
        } else {
            crate::error("interface recv ip packet failed.");
        }
    }
}

pub async fn create_hello_packet_raw_handle(
    send_packet_tx: broadcast::Sender<bytes::Bytes>,
    hello_interval: u16,
    network_mask: net::Ipv4Addr,
    options: u8,
    router_id: u32,
    area_id: u32,
    router_priority: u8,
    router_dead_interval: u32,
    designated_router: u32,
    backup_designated_router: u32,
    src_ip: net::Ipv4Addr,
    dst_ip: net::Ipv4Addr,
    neighbors: Arc<Mutex<HashMap<net::Ipv4Addr, neighbor::Neighbor>>>,
) {
    let duration = time::Duration::from_secs(hello_interval as u64);
    let ospf_packet_header = packet::OspfPacketHeader::new(
        OSPF_VERSION_2,
        packet::hello::HELLO_PACKET_TYPE,
        packet::OspfPacketHeader::length() as u16,
        router_id,
        area_id,
        0,
        0,
        0,
    );
    loop {
        time::sleep(duration).await;
        let hello_ospf_packet = packet::hello::HelloPacket::new(
            network_mask,
            hello_interval,
            options,
            router_priority,
            router_dead_interval,
            designated_router,
            backup_designated_router,
            ospf_packet_header,
            neighbors.clone(),
        );
        let mut ip_packet_buffer = vec![0u8; 1500];
        let ip_packet = new_ip_packet(
            ip_packet_buffer.as_mut_slice(),
            src_ip,
            dst_ip,
            hello_ospf_packet.to_bytes(),
        );
        if let Ok(ip_packet) = ip_packet {
            let hello_packet_bytes = bytes::Bytes::copy_from_slice(ip_packet.packet());
            if let Ok(_) = send_packet_tx.send(hello_packet_bytes) {
                crate::debug("send hello packet success.");
            } else {
                crate::error("send hello packet failed.");
            }
        } else {
            crate::error("hello packet to ip packet failed.");
        }
    }
}

pub async fn create_dd_packet_raw_handle() {}
