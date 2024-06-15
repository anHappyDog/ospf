use std::{
    collections::HashMap,
    mem::size_of,
    net,
    sync::{Arc, Mutex},
};

use pnet::{
    datalink,
    packet::ip::IpNextHeaderProtocols::Udp,
    transport::{self, TransportReceiver, TransportSender},
};
use tokio::{sync::broadcast, task::JoinHandle, time};

use crate::{
    debug, interface,
    packet::{hello::HELLO_PACKET_TYPE, try_get_from_ipv4_packet, OspfPacket},
    prompt_and_read, router, AllSPFRouters, OSPF_VERSION_2,
};
pub mod event;
pub mod handle;
pub mod status;

#[derive(Clone, Copy)]
pub enum InterfaceNetworkType {
    Broadcast,
    PointToPoint,
    NBMA,
    PointToMultipoint,
}

impl std::fmt::Display for InterfaceNetworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterfaceNetworkType::Broadcast => write!(f, "Broadcast"),
            InterfaceNetworkType::PointToPoint => write!(f, "PointToPoint"),
            InterfaceNetworkType::NBMA => write!(f, "NBMA"),
            InterfaceNetworkType::PointToMultipoint => write!(f, "PointToMultipoint"),
        }
    }
}

impl std::fmt::Debug for InterfaceNetworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterfaceNetworkType::Broadcast => write!(f, "Broadcast"),
            InterfaceNetworkType::PointToPoint => write!(f, "PointToPoint"),
            InterfaceNetworkType::NBMA => write!(f, "NBMA"),
            InterfaceNetworkType::PointToMultipoint => write!(f, "PointToMultipoint"),
        }
    }
}

pub struct Interface {
    pub name: String,
    pub ip_addr: net::Ipv4Addr,
    pub network_mask: net::Ipv4Addr,
    pub aread_id: net::Ipv4Addr,
    pub output_cost: u32,
    pub rxmt_interval: u32,
    pub inf_trans_delay: u32,
    pub router_priority: u32,
    pub hello_interval: u32,
    pub router_dead_interval: u32,
    pub auth_type: u32,
    pub auth_key: u64,
    pub network_type: InterfaceNetworkType,
    pub trans_rx: TransportReceiver,
    pub trans_tx: TransportSender,
    pub inner_rx: broadcast::Receiver<Arc<Mutex<dyn crate::packet::OspfPacket + Send>>>,
    pub inner_tx: broadcast::Sender<Arc<Mutex<dyn crate::packet::OspfPacket + Send>>>,
    pub send_packet_handle: Option<JoinHandle<()>>,
    pub recv_packet_handle: Option<JoinHandle<()>>,
    pub produce_hello_packet_handle: Option<JoinHandle<()>>,
    pub produce_dd_packet_handle: Option<JoinHandle<()>>,
    pub neighbors: Arc<Mutex<Vec<net::Ipv4Addr>>>,
    pub router: Arc<Mutex<crate::router::Router>>,
}

pub const DEFAULT_HELLO_INTERVAL: u32 = 10;
pub const DEFAULT_OUTPUT_COST: u32 = 1;
pub const DEFAULT_RXMT_INTERVAL: u32 = 5;
pub const DEFAULT_INF_TRANS_DELAY: u32 = 1;
pub const DEFAULT_ROUTER_PRIORITY: u32 = 1;
pub const DEFAULT_ROUTER_DEAD_INTERVAL: u32 = 40;
pub const DEFAULT_AUTH_TYPE: u32 = 0;
pub const DEFAULT_AUTH_KEY: u64 = 0;
pub const DEFAULT_AREA_ID: u32 = 0;

fn detect_pnet_interface() -> Result<Vec<datalink::NetworkInterface>, &'static str> {
    let interfaces = datalink::interfaces();
    if interfaces.len() == 0 {
        return Err("No interface found");
    }
    Ok(interfaces)
}

fn is_valid_pnet_interface(pnet_int: &datalink::NetworkInterface) -> bool {
    if pnet_int.is_loopback() || !pnet_int.is_up() {
        return false;
    }
    for ip in &pnet_int.ips {
        if let net::IpAddr::V4(_) = ip.ip() {
            if let net::IpAddr::V4(_) = ip.mask() {
                return true;
            }
        }
    }
    false
}

pub fn create_interfaces<'a>(
    router: Arc<Mutex<router::Router>>,
) -> Result<HashMap<String, Arc<Mutex<interface::Interface>>>, &'static str> {
    let pnet_ints = detect_pnet_interface()?;
    let mut ints = HashMap::new();
    for int in pnet_ints {
        if !is_valid_pnet_interface(&int) {
            continue;
        }
        println!("------------------------------------------");
        println!("detect interface: {}", int.name);
        let area_id: u32 = prompt_and_read(&format!(
            "please input the area id(default is {},press enter):",
            DEFAULT_AREA_ID
        ))
        .parse()
        .unwrap_or(DEFAULT_AREA_ID);
        let output_cost = prompt_and_read(&format!(
            "please input the output cost(default is {}):",
            DEFAULT_OUTPUT_COST
        ))
        .parse()
        .unwrap_or(DEFAULT_OUTPUT_COST);
        let rxmt_interval = prompt_and_read(&format!(
            "please input the rxmt interval(default is {}):",
            DEFAULT_RXMT_INTERVAL
        ))
        .parse()
        .unwrap_or(DEFAULT_RXMT_INTERVAL);
        let inf_trans_delay = prompt_and_read(&format!(
            "please input the inf_trans_delay(default is {}):",
            DEFAULT_INF_TRANS_DELAY
        ))
        .parse()
        .unwrap_or(DEFAULT_INF_TRANS_DELAY);
        let router_priority = prompt_and_read(&format!(
            "please input the router priority(default is {}):",
            DEFAULT_ROUTER_PRIORITY
        ))
        .parse()
        .unwrap_or(DEFAULT_ROUTER_PRIORITY);
        let hello_interval = prompt_and_read(&format!(
            "please input the hello interval(default is {}):",
            DEFAULT_HELLO_INTERVAL
        ))
        .parse()
        .unwrap_or(DEFAULT_HELLO_INTERVAL);
        let router_dead_interval = prompt_and_read(&format!(
            "please input the router dead interval(default is {}):",
            DEFAULT_ROUTER_DEAD_INTERVAL
        ))
        .parse()
        .unwrap_or(DEFAULT_ROUTER_DEAD_INTERVAL);
        let auth_type = prompt_and_read(&format!(
            "please input the auth type(default is {}):",
            DEFAULT_AUTH_TYPE
        ))
        .parse()
        .unwrap_or(DEFAULT_AUTH_TYPE);
        let auth_key = prompt_and_read(&format!(
            "please input the auth key(default is {},PS: just for test):",
            DEFAULT_AUTH_KEY
        ))
        .parse()
        .unwrap_or(DEFAULT_AUTH_KEY);

        if let Some(int) = Interface::from_pnet_interface(
            router.clone(),
            &int,
            net::Ipv4Addr::from(area_id),
            output_cost,
            rxmt_interval,
            inf_trans_delay,
            router_priority,
            hello_interval,
            router_dead_interval,
            auth_type,
            auth_key,
        ) {
            ints.insert(int.name.clone(), Arc::new(Mutex::new(int)));
        }
    }
    println!("----------------- all interfaces are set,adding to router -------");
    Ok(ints)
}

async fn int_recv_packet<'a>(rx: &mut transport::TransportReceiver) -> u32 {
    let mut recv_packet_count = 0;
    let mut packet_iter = transport::ipv4_packet_iter(rx);

    loop {
        if let Ok((packet, _)) = packet_iter.next() {
            if packet.get_dscp() == crate::OSPF_IP_PROTOCOL_NUMBER {
                debug(&format!("recv ospf packet."));
                let possible_neighbors = Arc::new(Mutex::new(Vec::new()));
                let ospf_packet = try_get_from_ipv4_packet(&packet, possible_neighbors);
                let ospf_packet = match ospf_packet {
                    Ok(p) => p,
                    Err(e) => {
                        debug(&format!("recv packet, but not ospf packet.{}", e));
                        continue;
                    }
                };
                debug(&format!(
                    "recv ospf [type is {}] packet",
                    ospf_packet.lock().unwrap().get_type()
                ));
            } else {
                debug(&format!("recv packet, but not ospf packet."));
            }
        } else {
            debug(&format!("recv non-ipv4 packet."));
            continue;
        }
        recv_packet_count += 1;
    }

    recv_packet_count
}

async fn int_send_packet<'a>(
    tx: &mut transport::TransportSender,
    hello_interval: u16,
    ip_addr: net::Ipv4Addr,
    router_id: u32,
    area_id: u32,
    auth_type: u8,
    netwok_mask: net::Ipv4Addr,
    router_priority: u8,
    router_dead_interval: u32,
    neighbors: Arc<Mutex<Vec<net::Ipv4Addr>>>,
    network_type: InterfaceNetworkType,
) -> u32 {
    let mut send_packet_count = 0;
    let hello_send_interval = time::Duration::from_secs(hello_interval as u64);
    let default_ospf_header_length = size_of::<crate::packet::OspfPacketHeader>();
    loop {
        time::sleep(hello_send_interval).await;
        let ospf_packet_header = crate::packet::OspfPacketHeader::new(
            OSPF_VERSION_2,
            HELLO_PACKET_TYPE,
            default_ospf_header_length as u16,
            router_id,
            area_id,
            0,
            auth_type,
            0,
        );
        let mut ospf_hello_packet = crate::packet::hello::HelloPacket::new(
            netwok_mask,
            hello_interval,
            0,
            router_priority,
            router_dead_interval,
            0,
            0,
            ospf_packet_header,
            neighbors.clone(),
        );
        ospf_hello_packet.calculate_checksum();
        match network_type {
            InterfaceNetworkType::Broadcast => {
                let _ = crate::packet::send_to(&ospf_hello_packet, tx, ip_addr, AllSPFRouters);
            }
            InterfaceNetworkType::PointToPoint => {
                let _ = crate::packet::send_to(&ospf_hello_packet, tx, ip_addr, AllSPFRouters);
            }
            _ => {
                break;
            }
        }
        send_packet_count += 1;
    }

    send_packet_count
}

impl Interface {
    pub const INNER_PACKET_QUEUE_SIZE: u32 = 128;

    pub fn get_neighbors(&self) -> Arc<Mutex<Vec<net::Ipv4Addr>>> {
        self.neighbors.clone()
    }

    pub fn get_area_id(&self) -> net::Ipv4Addr {
        self.aread_id
    }
    /// init the interfaces' handlers
    pub async fn init_handlers(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let hello_interval = self.hello_interval;
        let ip_addr = self.ip_addr;
        let router = self.router.clone();
        let router_id = router.lock().unwrap().get_router_id();

        let area_id = self.aread_id;
        let auth_type = self.auth_type;
        let network_type = self.network_type;
        let network_mask = self.network_mask;
        let router_priority = self.router_priority;
        let router_dead_interval = self.router_dead_interval;
        let neighbors: Arc<Mutex<Vec<net::Ipv4Addr>>> = self.neighbors.clone();

        let (trans_tx, trans_rx) =
            transport::transport_channel(1500, transport::TransportChannelType::Layer3(Udp))
                .unwrap();

        let (inner_tx, inner_rx) = broadcast::channel::<Arc<Mutex<dyn OspfPacket + Send>>>(
            Interface::INNER_PACKET_QUEUE_SIZE as usize,
        );

        self.send_packet_handle = Some(tokio::spawn(handle::create_send_packet_handle(
            inner_rx,
            trans_tx,
            ip_addr,
            AllSPFRouters,
            network_type,
        )));
        self.recv_packet_handle = Some(tokio::spawn(handle::create_recv_packet_handle(
            trans_rx,
            inner_tx.clone(),
        )));
        self.produce_hello_packet_handle = Some(tokio::spawn(handle::create_hello_packet_handle(
            inner_tx,
            neighbors.clone(),
            hello_interval as u64,
            network_mask,
            router_priority as u8,
            router_dead_interval,
            0,
            router_id.to_bits(),
            area_id.to_bits(),
            auth_type as u8,
        )));
        self.produce_dd_packet_handle = Some(tokio::spawn(handle::create_dd_packet_handle()));
        Ok(())
    }
    pub fn from_pnet_interface(
        router: Arc<Mutex<router::Router>>,
        pnet_int: &datalink::NetworkInterface,
        aread_id: net::Ipv4Addr,
        output_cost: u32,
        rxmt_interval: u32,
        inf_trans_delay: u32,
        router_prioriry: u32,
        hello_interval: u32,
        router_dead_interval: u32,
        auth_type: u32,
        auth_key: u64,
    ) -> Option<Self> {
        if pnet_int.is_loopback() || !pnet_int.is_up() {
            return None;
        }
        let mut ip_addr = net::Ipv4Addr::new(255, 255, 255, 255); //false addr
        let mut network_mask = net::Ipv4Addr::new(255, 255, 255, 255);
        let mut network_type = InterfaceNetworkType::Broadcast;
        let mut found_ip_flag = false;
        for ip in &pnet_int.ips {
            if let net::IpAddr::V4(taddr) = ip.ip() {
                if let net::IpAddr::V4(tmask) = ip.mask() {
                    ip_addr = taddr;
                    network_mask = tmask;
                    found_ip_flag = true;
                    if pnet_int.is_point_to_point() {
                        network_type = InterfaceNetworkType::PointToPoint;
                    } else if pnet_int.is_multicast() {
                        network_type = InterfaceNetworkType::PointToMultipoint;
                    } else if pnet_int.is_broadcast() {
                        network_type = InterfaceNetworkType::Broadcast;
                    } else {
                        network_type = InterfaceNetworkType::NBMA;
                    }
                    break;
                }
            }
        }
        if !found_ip_flag {
            return None;
        }
        let name = pnet_int.name.clone();
        println!("interface [{}] set.", name);
        println!("interface ipv4 addr: {}", ip_addr);
        println!("interface network mask: {}", network_mask);

        let (inner_tx, inner_rx) = broadcast::channel::<Arc<Mutex<dyn OspfPacket + Send>>>(
            Interface::INNER_PACKET_QUEUE_SIZE as usize,
        );
        let (trans_tx, trans_rx) =
            transport::transport_channel(1500, transport::TransportChannelType::Layer3(Udp))
                .unwrap();

        let int = Self::new(
            ip_addr,
            network_mask,
            aread_id,
            output_cost,
            rxmt_interval,
            inf_trans_delay,
            router_prioriry,
            hello_interval,
            router_dead_interval,
            auth_type,
            auth_key,
            name,
            network_type,
            inner_tx,
            inner_rx,
            trans_tx,
            trans_rx,
            router,
        );
        Some(int)
    }
    pub fn set_hello_interval(&mut self, hello_interval: u32) {
        self.hello_interval = hello_interval;
    }
    pub fn set_router_priority(&mut self, router_prioriry: u32) {
        self.router_priority = router_prioriry;
    }
    pub fn set_inf_trans_delay(&mut self, inf_trans_delay: u32) {
        self.inf_trans_delay = inf_trans_delay;
    }
    pub fn set_rxmt_interval(&mut self, rxmt_interval: u32) {
        self.rxmt_interval = rxmt_interval;
    }

    pub fn new(
        ip_addr: net::Ipv4Addr,
        network_mask: net::Ipv4Addr,
        aread_id: net::Ipv4Addr,
        output_cost: u32,
        rxmt_interval: u32,
        inf_trans_delay: u32,
        router_prioriry: u32,
        hello_interval: u32,
        router_dead_interval: u32,
        auth_type: u32,
        auth_key: u64,
        name: String,
        network_type: InterfaceNetworkType,
        inner_tx: broadcast::Sender<Arc<Mutex<dyn crate::packet::OspfPacket + Send>>>,
        inner_rx: broadcast::Receiver<Arc<Mutex<dyn crate::packet::OspfPacket + Send>>>,
        trans_tx: transport::TransportSender,
        trans_rx: transport::TransportReceiver,
        router: Arc<Mutex<router::Router>>,
    ) -> Self {
        Self {
            name,
            ip_addr,
            network_mask,
            aread_id,
            output_cost,
            rxmt_interval,
            inf_trans_delay,
            router_priority: router_prioriry,
            hello_interval,
            router_dead_interval,
            auth_type,
            auth_key,
            network_type,
            send_packet_handle: None,
            recv_packet_handle: None,
            produce_dd_packet_handle: None,
            produce_hello_packet_handle: None,
            neighbors: Arc::new(Mutex::new(Vec::new())),
            router: router,
            inner_tx,
            inner_rx,
            trans_rx,
            trans_tx,
        }
    }
}
