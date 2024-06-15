use std::{
    net,
    sync::{Arc, Mutex},
};

use pnet::transport::{self, ipv4_packet_iter};
use tokio::{sync::broadcast, time};

use crate::{
    lsa::router,
    packet::{self, send_to, OspfPacket},
    OSPF_VERSION_2,
};

use super::InterfaceNetworkType;

pub(super) async fn create_send_packet_handle(
    mut inner_rx: broadcast::Receiver<Arc<Mutex<dyn packet::OspfPacket + Send>>>,
    mut trans_tx: transport::TransportSender,
    src_ip: net::Ipv4Addr,
    dst_ip: net::Ipv4Addr,
    network_type: InterfaceNetworkType,
) {
    loop {
        if let Ok(packet) = inner_rx.recv().await {
            let packet = packet.lock().unwrap();

            if let Ok(_) = send_to(&*packet, &mut trans_tx, src_ip, dst_ip) {
                println!("send packet success");
            } else {
                println!("send packet failed");
            }
        } else {
            println!("inner_rx recv failed");
        }
    }
}

pub(super) async fn create_recv_packet_handle(
    mut trans_rx: transport::TransportReceiver,
    inner_tx: broadcast::Sender<Arc<Mutex<dyn packet::OspfPacket + Send>>>,
) {
    let mut ipv4_packet_iter = ipv4_packet_iter(&mut trans_rx);
    loop {
        if let Ok((packet, _ip)) = ipv4_packet_iter.next() {
            let hello_neighbors = Arc::new(Mutex::new(Vec::new()));
            if let Ok(ospf_packet) = packet::try_get_from_ipv4_packet(&packet, hello_neighbors) {
                if let Ok(_) = inner_tx.send(ospf_packet) {
                    println!("inner_tx send success");
                } else {
                    println!("inner_tx send failed");
                }
            } else {
                println!("try_get_from_ipv4_packet failed");
            }
        } else {
            println!("ipv4_packet_iter next failed");
        }
    }
}

pub(super) async fn create_hello_packet_handle(
    inner_tx: broadcast::Sender<Arc<Mutex<dyn packet::OspfPacket + Send>>>,
    neighbors: Arc<Mutex<Vec<net::Ipv4Addr>>>,
    hello_interval: u64,
    network_mask: net::Ipv4Addr,
    router_priority: u8,
    router_dead_interval: u32,
    options: u8,

    router_id: u32,
    area_id: u32,
    auth_type: u8,
) {
    let hello_duration = time::Duration::from_secs(hello_interval as u64);
    let designated_router = 0;
    let backup_designated_router = 0;
    let ospf_packet_header = packet::OspfPacketHeader::new(
        OSPF_VERSION_2,
        packet::hello::HELLO_PACKET_TYPE,
        packet::OspfPacketHeader::length() as u16,
        router_id,
        area_id,
        0,
        auth_type,
        0,
    );

    loop {
        time::sleep(hello_duration).await;

        let hello_packet = packet::hello::HelloPacket::new(
            network_mask,
            hello_interval as u16,
            options,
            router_priority as u8,
            router_dead_interval,
            designated_router,
            backup_designated_router,
            ospf_packet_header,
            neighbors.clone(),
        );
        match inner_tx.send(Arc::new(Mutex::new(hello_packet))) {
            Ok(_) => {}
            Err(e) => {}
        }
    }
}

pub(super) async fn create_dd_packet_handle() {}
