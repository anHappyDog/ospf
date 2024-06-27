use std::net;

pub mod area;
pub mod cli;
pub mod interface;
pub mod lsa;
pub mod neighbor;
pub mod packet;
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
    Ok(())
}