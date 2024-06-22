use std::{
    collections::HashMap,
    net,
    sync::{Arc, Mutex},
};

use pnet::{
    packet::{
        ip::IpNextHeaderProtocol,
        ipv4::{self, Ipv4Packet},
        udp, Packet,
    },
    transport::{self, ipv4_packet_iter},
};
use tokio::{sync::broadcast, task::JoinHandle, time};

use crate::{
    error,
    lsa::router,
    neighbor,
    packet::{self, is_ospf_packet_valid, new_ip_packet, OspfPacket},
    OSPF_IP_PROTOCOL_NUMBER, OSPF_VERSION_2,
};

pub struct InterfaceHandler {
    pub send_tcp_packet_handle: Option<JoinHandle<()>>,
    pub recv_tcp_packet_handle: Option<JoinHandle<()>>,
    pub send_udp_packet_handle: Option<JoinHandle<()>>,
    pub recv_udp_packet_handle: Option<JoinHandle<()>>,
    pub hello_timer_handle: Option<JoinHandle<()>>,
    pub dd_timer_handle: Option<JoinHandle<()>>,
    pub wait_timer_handle: Option<JoinHandle<()>>,
}

pub fn add_interface_handlers(ip_addr: net::IpAddr) -> Arc<Mutex<InterfaceHandler>> {
    let interface_handlers = crate::INTERFACE_HANDLERS.clone();
    let mut locked_interface_handlers = interface_handlers.lock().unwrap();

    let handlers = Arc::new(Mutex::new(InterfaceHandler {
        send_tcp_packet_handle: None,
        recv_tcp_packet_handle: None,
        send_udp_packet_handle: None,
        recv_udp_packet_handle: None,
        hello_timer_handle: None,
        dd_timer_handle: None,
        wait_timer_handle: None,
    }));
    locked_interface_handlers.insert(ip_addr, handlers.clone());
    handlers
}

pub async fn send_tcp_packet(
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

pub async fn send_udp_packet(
    mut to_send_udp_packet_rx: broadcast::Receiver<bytes::Bytes>,
    mut udp_tx: transport::TransportSender,
) {
    loop {
        if let Ok(packet_bytes) = to_send_udp_packet_rx.recv().await {
            if let Some(packet) = Ipv4Packet::new(&packet_bytes) {
                let destination = packet.get_destination();
                if let Ok(_) = udp_tx.send_to(packet, net::IpAddr::V4(destination)) {
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

pub async fn recv_tcp_packet(
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
/// # recv_udp_packet
///this function is used to handle the received udp packet from the interface.
///it can receive udp and tcp packet, if received ospf packet, then handle it.
///otherwise, just forwarding it or receive it.
///
pub async fn recv_udp_packet(
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

pub async fn create_wait_timer(router_dead_interval: u32) {
    let duration = time::Duration::from_secs(router_dead_interval as u64);
    time::sleep(duration).await;
}

pub async fn create_hello_packet(
    send_packet_tx: broadcast::Sender<bytes::Bytes>,
    ip_addr: net::Ipv4Addr,
    dst_ip: net::Ipv4Addr,
) {
    let interfaces = crate::INTERFACES.clone();
    let locked_interfaces = interfaces.lock().unwrap();
    let interface = locked_interfaces
        .get(&ip_addr.into())
        .expect("get interface failed.");
    let locked_interface = interface.lock().expect("get interface lock failed.");
    let hello_interval = locked_interface.hello_interval;
    let router_id = crate::ROUTER_ID.clone();
    let area_id = locked_interface.aread_id;
    let network_mask = locked_interface.network_mask;
    let options = 0;
    let router_priority = locked_interface.router_priority;
    let router_dead_interval = locked_interface.router_dead_interval;
    let duration = time::Duration::from_secs(hello_interval as u64);
    drop(locked_interface);
    drop(locked_interfaces);
    let ospf_packet_header = packet::OspfPacketHeader::new(
        OSPF_VERSION_2,
        packet::hello::HELLO_PACKET_TYPE,
        packet::OspfPacketHeader::length() as u16,
        router_id.into(),
        area_id.into(),
        0,
        0,
        0,
    );
    loop {
        time::sleep(duration).await;
        let interfaces = crate::INTERFACES.clone();
        let locked_interfaces = interfaces.lock().unwrap();
        let interface = locked_interfaces
            .get(&ip_addr.into())
            .expect("get interface failed.");
        let locked_interface = interface.lock().expect("get interface lock failed.");
        let designated_router = locked_interface.designated_router;
        let backup_designated_router = locked_interface.backup_designated_router;
        let neighbors = crate::NEIGHBORS.clone();
        let locked_neighbors = neighbors.lock().unwrap();
        let interface_neighbors = locked_neighbors.get(&ip_addr).unwrap();

        let hello_ospf_packet = packet::hello::HelloPacket::new(
            network_mask,
            hello_interval as u16,
            options,
            router_priority as u8,
            router_dead_interval,
            designated_router.into(),
            backup_designated_router.into(),
            ospf_packet_header,
            interface_neighbors.clone(),
        );
        let mut ip_packet_buffer = vec![0u8; crate::MTU];
        let ip_packet = new_ip_packet(
            ip_packet_buffer.as_mut_slice(),
            ip_addr,
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

pub async fn create_dd_packet() {}
