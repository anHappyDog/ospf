use std::{collections::HashMap, future::Future, net, sync::Arc};

use pnet::{
    packet::{
        ip::{
            IpNextHeaderProtocol,
            IpNextHeaderProtocols::{Tcp, Udp},
        },
        ipv4, Packet,
    },
    transport,
};
use tokio::sync::{broadcast, RwLock};

use crate::{
    err,
    packet::{
        dd::DD_TYPE, hello::HELLO_TYPE, lsack::LSACK_TYPE, lsr::LSR_TYPE, lsu::LSU_TYPE,
        IPV4_PACKET_MTU,
    },
    OSPF_IP_PROTOCOL,
};

/// # Handler
/// the data structure is used to store the handler for the interface
/// the handler contains the JoinHandle for send_tcp,send_udp,recv_tcp,recv_udp
/// wait_timer and hello_timer
pub struct Handler {
    pub send_tcp:
        Option<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>>,
    pub send_udp:
        Option<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>>,
    pub recv_tcp:
        Option<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>>,
    pub recv_udp:
        Option<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>>,
    pub wait_timer:
        Option<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>>,
    pub hello_timer:
        Option<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>>,
}

pub async fn init_when_interface_up(
    ipv4_addr: net::Ipv4Addr,
    interface_name: String,
    network_type: super::NetworkType,
    router_priority: u8,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (tcp_tx, tcp_rx) = match transport::transport_channel(
        IPV4_PACKET_MTU,
        transport::TransportChannelType::Layer3(Tcp),
    ) {
        Ok((tx, rx)) => (tx, rx),
        Err(e) => {
            crate::util::error(&format!("create tcp channel failed:{}", e));
            return Err(Box::new(err::OspfError::new(format!(
                "create tcp channel failed:{}",
                e
            ))));
        }
    };
    let (udp_tx, udp_rx) = match transport::transport_channel(
        IPV4_PACKET_MTU,
        transport::TransportChannelType::Layer3(Udp),
    ) {
        Ok((tx, rx)) => (tx, rx),
        Err(e) => {
            crate::util::error(&format!("create udp channel failed:{}", e));
            return Err(Box::new(err::OspfError::new(format!(
                "create udp channel failed:{}",
                e
            ))));
        }
    };
    let handlers = HANDLERS.read().await;
    let transmissions = super::trans::TRANSMISSIONS.write().await;
    let tcp_inner_rx = match transmissions.get(&ipv4_addr) {
        Some(transmission) => transmission.write().await.inner_tcp_tx.clone().subscribe(),
        None => {
            crate::util::error("transmission not found.");
            return Err(Box::new(err::OspfError::new(
                "transmission not found.".to_string(),
            )));
        }
    };
    let udp_inner_rx = match transmissions.get(&ipv4_addr) {
        Some(transmission) => transmission.write().await.inner_udp_tx.subscribe(),
        None => {
            crate::util::error("transmission not found.");
            return Err(Box::new(err::OspfError::new(
                "transmission not found.".to_string(),
            )));
        }
    };
    let udp_inner_tx = match transmissions.get(&ipv4_addr) {
        Some(transmission) => transmission.write().await.inner_udp_tx.clone(),
        None => {
            crate::util::error("transmission not found.");
            return Err(Box::new(err::OspfError::new(
                "transmission not found.".to_string(),
            )));
        }
    };
    let mut handler = match handlers.get(&ipv4_addr) {
        Some(handler) => handler.write().await,
        None => {
            crate::util::error("handler not found.");
            return Err(Box::new(err::OspfError::new(
                "handler not found.".to_string(),
            )));
        }
    };
    handler.send_tcp = Some(tokio::spawn(send_tcp(tcp_tx, tcp_inner_rx)));
    handler.send_udp = Some(tokio::spawn(send_udp(udp_tx, udp_inner_rx)));
    handler.recv_tcp = Some(tokio::spawn(recv_tcp(tcp_rx)));
    handler.recv_udp = Some(tokio::spawn(recv_udp(
        udp_rx,
        interface_name.clone(),
        ipv4_addr.clone(),
    )));
    handler.hello_timer = Some(tokio::spawn(hello_timer(ipv4_addr, udp_inner_tx.clone())));
    match network_type {
        super::NetworkType::Broadcast | super::NetworkType::NBMA => {
            if router_priority != 0 {
                handler.wait_timer = Some(tokio::spawn(wait_timer()));
            }
        }
        _ => {}
    }
    Ok(())
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

pub fn try_get_ospf_packet(
    ipv4_packet: &ipv4::Ipv4Packet,
    interface_name: String,
) -> Result<crate::packet::OspfPacket, &'static str> {
    match ipv4_packet.get_next_level_protocol() {
        IpNextHeaderProtocol(OSPF_IP_PROTOCOL) => {
            let payload = ipv4_packet.payload();
            match crate::packet::OspfHeader::try_from_be_bytes(payload) {
                Some(ospf_header) => match ospf_header.packet_type {
                    HELLO_TYPE => {
                        let hello_packet = crate::packet::hello::Hello::try_from_be_bytes(payload);
                        match hello_packet {
                            Some(hello_packet) => {
                                crate::util::debug("ospf hello packet received.");
                                Ok(crate::packet::OspfPacket::Hello(hello_packet))
                            }
                            None => Err("invalid hello packet,ignored."),
                        }
                    }
                    DD_TYPE => {
                        let dd_packet = crate::packet::dd::DD::try_from_be_bytes(payload);
                        match dd_packet {
                            Some(dd_packet) => {
                                crate::util::debug("ospf dd packet received.");
                                Ok(crate::packet::OspfPacket::DD(dd_packet))
                            }
                            None => Err("invalid dd packet,ignored."),
                        }
                    }
                    LSR_TYPE => {
                        let lsr_packet = crate::packet::lsr::Lsr::try_from_be_bytes(payload);
                        match lsr_packet {
                            Some(lsr_packet) => {
                                crate::util::debug("ospf lsr packet received.");
                                Ok(crate::packet::OspfPacket::LSR(lsr_packet))
                            }
                            None => Err("invalid lsr packet,ignored."),
                        }
                    }
                    LSU_TYPE => {
                        let lsu_packet = crate::packet::lsu::Lsu::try_from_be_bytes(payload);
                        match lsu_packet {
                            Some(lsu_packet) => {
                                crate::util::debug("ospf lsu packet received.");
                                Ok(crate::packet::OspfPacket::LSU(lsu_packet))
                            }
                            None => Err("invalid lsu packet,ignored."),
                        }
                    }
                    LSACK_TYPE => {
                        let lsack_packet = crate::packet::lsack::Lsack::try_from_be_bytes(payload);
                        match lsack_packet {
                            Some(lsack_packet) => {
                                crate::util::debug("ospf lsack packet received.");
                                Ok(crate::packet::OspfPacket::LSACK(lsack_packet))
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
    interface_name: String,
    ipv4_addr: net::Ipv4Addr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut ipv4_packet_iter = transport::ipv4_packet_iter(&mut udp_rx);
    loop {
        match ipv4_packet_iter.next() {
            Ok((ipv4_packet, _)) => {
                if !is_ipv4_packet_valid(&ipv4_packet) {
                    crate::util::error("invalid ipv4 packet.");
                    continue;
                }
                match try_get_ospf_packet(&ipv4_packet, interface_name.clone()) {
                    Ok(ospf_packet) => match ospf_packet {
                        crate::packet::OspfPacket::Hello(hello_packet) => {
                            crate::util::debug("ospf hello packet received.");
                            if !crate::packet::is_ipv4_packet_valid_for_ospf(
                                &ipv4_packet,
                                ipv4_addr,
                            ) {
                                crate::util::error("Invalid packet for ospf received.");
                                continue;
                            }
                            let source_addr = ipv4_packet.get_source();
                            tokio::spawn(crate::packet::hello::when_received(
                                source_addr.clone(),
                                hello_packet.clone(),
                                interface_name.clone(),
                            ));
                        }
                        crate::packet::OspfPacket::DD(dd_packet) => {
                            crate::util::debug("ospf dd packet received.");
                            tokio::spawn(crate::packet::dd::when_received(dd_packet));
                        }
                        crate::packet::OspfPacket::LSR(lsr_packet) => {
                            crate::util::debug("ospf lsr packet received.");
                            tokio::spawn(crate::packet::lsr::when_received(lsr_packet));
                        }
                        crate::packet::OspfPacket::LSU(lsu_packet) => {
                            crate::util::debug("ospf lsu packet received.");
                            tokio::spawn(crate::packet::lsu::when_received(lsu_packet));
                        }
                        crate::packet::OspfPacket::LSACK(lsack_packet) => {
                            crate::util::debug("ospf lsack packet received.");
                            tokio::spawn(crate::packet::lsack::when_received(lsack_packet));
                        }
                    },
                    Err(e) => {
                        crate::util::debug(&format!("{},ignored.", e));

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
pub async fn recv_tcp(
    mut tcp_rx: transport::TransportReceiver,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

pub async fn wait_timer() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
}

/// # hello_timer
/// the function is used to create the future handle for hello timer
/// it will keep creating hello packet and send it to the sender tx by
/// the interval of hello interval.
/// - ipv4_addr : the ipv4 address of the interface
pub async fn hello_timer(
    ipv4_addr: net::Ipv4Addr,
    udp_inner_tx: broadcast::Sender<bytes::Bytes>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let interfaces = super::INTERFACES.read().await;
    let interface = match interfaces.get(&ipv4_addr) {
        Some(interface) => interface.read().await,
        None => {
            crate::util::error("interface not found.");
            return Err(Box::new(err::OspfError::new(
                "interface not found.".to_string(),
            )));
        }
    };
    let hello_interval = interface.hello_interval;
    let network_mask = interface.mask;
    let interval = tokio::time::Duration::from_secs(hello_interval as u64);
    let options = interface.options;
    let router_priority = interface.router_priority;
    let router_dead_interval = interface.router_dead_interval;
    let network_type = interface.network_type;
    let area_id = interface.area_id;
    let mut buffer: Vec<u8> = vec![0; IPV4_PACKET_MTU as usize];
    drop(interface);
    drop(interfaces);
    loop {
        tokio::time::sleep(interval).await;
        let interfaces = super::INTERFACES.read().await;
        let interface = match interfaces.get(&ipv4_addr) {
            Some(interface) => interface.read().await,
            None => {
                crate::util::error("interface not found.");
                return Err(Box::new(err::OspfError::new(
                    "interface not found.".to_string(),
                )));
            }
        };

        let designated_router = interface.designated_router;
        let backup_designated_router = interface.backup_designated_router;

        drop(interface);
        drop(interfaces);
        let mut hello_packet = crate::packet::hello::Hello::empty();
        hello_packet.header.router_id = crate::ROUTER_ID.clone().into();
        hello_packet.header.packet_type = HELLO_TYPE;
        hello_packet.header.packet_length = hello_packet.length() as u16;
        hello_packet.header.area_id = area_id.into();
        hello_packet.header.auth_type = 0;
        hello_packet.header.authentication = [0; 8];
        hello_packet.header.version = crate::OSPF_VERSION;

        hello_packet.network_mask = network_mask.into();
        hello_packet.hello_interval = hello_interval;
        hello_packet.options = options;
        hello_packet.router_priority = router_priority;
        hello_packet.router_dead_interval = router_dead_interval;
        hello_packet.designated_router = designated_router.into();
        hello_packet.backup_designated_router = backup_designated_router.into();
        hello_packet.header.checksum =
            crate::packet::ospf_packet_checksum(&hello_packet.to_be_bytes());
        println!("hello_packet created : {:#?}", hello_packet);
        let neighbors = crate::neighbor::NEIGHBORS.read().await;
        let int_neighbors = match neighbors.get(&ipv4_addr) {
            Some(neighbors) => neighbors.read().await,
            None => {
                crate::util::error("neighbors not found.");
                return Err(Box::new(err::OspfError::new(
                    "neighbors not found.".to_string(),
                )));
            }
        };
        int_neighbors
            .keys()
            .into_iter()
            .for_each(|neighbor_ipv4_addr| {
                hello_packet
                    .neighbors
                    .push(neighbor_ipv4_addr.clone().into());
            });
        drop(int_neighbors);
        drop(neighbors);

        let hello_ipv4_packet = loop {
            crate::util::debug(&format!(
                "build hello packet,size of buffer:{},hello_interval : {}.",
                hello_packet.length(),
                hello_packet.hello_interval
            ));
            match hello_packet.build_ipv4_packet(&mut buffer, ipv4_addr, network_type) {
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
                    break;
                }
                Err(e) => {
                    crate::util::error(&format!("send hello packet failed:{}", e));
                }
            }
        }
    }
}

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
