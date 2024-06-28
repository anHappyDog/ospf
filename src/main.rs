use std::net;

use packet::{hello::HELLO_TYPE, ospf_packet_checksum};
use pnet::{packet::ip::IpNextHeaderProtocols, transport::{self, TransportChannelType}};

pub mod area;
pub mod cli;
pub mod interface;
pub mod lsa;
pub mod neighbor;
pub mod packet;
pub mod test;
pub mod rtable;
pub mod util;

pub const OPTION_E: u8 = 0x02;
pub const OPTION_MC: u8 = 0x04;
pub const OPTION_NP: u8 = 0x08;
pub const OPTION_EA: u8 = 0x10;
pub const OPTION_DC: u8 = 0x20;

lazy_static::lazy_static! {
    pub static ref ROUTER_ID : net::Ipv4Addr = util::input_router_id();
}

pub const ALL_SPF_ROUTERS: net::Ipv4Addr = net::Ipv4Addr::new(224, 0, 0, 5);
pub const ALL_DROTHERS: net::Ipv4Addr = net::Ipv4Addr::new(224, 0, 0, 6);

pub const OSPF_IP_PROTOCOL: u8 = 89;
pub const OSPF_VERSION: u8 = 2;

pub const IPV4_PACKET_MTU: usize = 1500;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    interface::init().await?;
    cli::cli().await?;
    // test_send().await;
    Ok(())
}


async fn test_send() {
    let (mut tx,rx) = match transport::transport_channel(1500, TransportChannelType::Layer3(IpNextHeaderProtocols::Ipv4)) {
        Ok((tx,rx)) => (tx,rx),
        _ => {
            return;
        }
    };
    let mut  hello_packet = crate::packet::hello::Hello::empty();
    hello_packet.header.area_id = net::Ipv4Addr::new(0, 2, 2, 0);
    hello_packet.header.router_id = net::Ipv4Addr::new(1, 2, 3, 4);
    hello_packet.header.version = crate::OSPF_VERSION;
    hello_packet.header.auth_type = 0;
    hello_packet.header.packet_type= HELLO_TYPE;
    hello_packet.header.authentication = 0;
    hello_packet.header.packet_length = 0;
    hello_packet.hello_interval = 10;
    hello_packet.options = OPTION_E;
    hello_packet.router_priority = 1;
    hello_packet.router_dead_interval = 40;
    hello_packet.designated_router = net::Ipv4Addr::new(1, 2, 3, 4).into();
    hello_packet.backup_designated_router = net::Ipv4Addr::new(1, 2, 3, 4).into();
    hello_packet.neighbors.push(net::Ipv4Addr::new(1, 2, 3, 4).into());
    hello_packet.neighbors.push(net::Ipv4Addr::new(1, 2, 3, 4).into());
    hello_packet.network_mask = net::Ipv4Addr::new(255, 255, 255, 0).into();
    hello_packet.header.packet_length = hello_packet.length() as u16;
    hello_packet.header.checksum = ospf_packet_checksum(&hello_packet.to_be_bytes());
    let mut buffer = vec![0; crate::IPV4_PACKET_MTU];
    let ip_packet = hello_packet.build_ipv4_packet(&mut buffer, net::Ipv4Addr::new(172,17,137,183)).await.unwrap();
    tx.send_to(ip_packet, ALL_SPF_ROUTERS.into()).unwrap();

}
