use std::{collections::HashMap, net, sync::Arc};

use pnet::{packet::ipv4, transport};
use tokio::sync::{broadcast, RwLock};

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

/// # recv_udp
/// the function is used to create the future handle for recv udp ipv4 packet
/// - udp_rx : the receiver for the udp handler
/// - udp_inner_tx : the sender for inner interface or other interfaces, to forward the ipv4 packet
/// the function will receive the ipv4 packet from the udp handler and forward the packet to the inner interface or other interfaces
/// the function will loop until the ipv4 packet is received
pub async fn recv_udp(
    mut udp_rx: transport::TransportReceiver,
    mut udp_inner_tx: broadcast::Sender<bytes::Bytes>,
) {
    let mut ipv4_packet_iter = transport::ipv4_packet_iter(&mut udp_rx);
    loop {
        match ipv4_packet_iter.next() {
            Ok((ipv4_packet, ip)) => {}
            Err(_) => {
                continue;
            }
        }
    }
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
            Ok((ipv4_packet, ip)) => {}
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
