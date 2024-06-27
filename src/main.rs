use std::net;

pub mod interface;
pub mod util;
pub mod cli;
pub mod area;
pub mod packet;
pub mod lsa;
pub mod neighbor;
pub mod rtable;


lazy_static::lazy_static! {
    pub static ref ROUTER_ID : net::Ipv4Addr = util::input_router_id();
}

pub const ALL_SPF_ROUTERS: net::Ipv4Addr = net::Ipv4Addr::new(224, 0, 0, 5);
pub const ALL_DROTHERS: net::Ipv4Addr = net::Ipv4Addr::new(224, 0, 0, 6);

pub const OSPF_IP_PROTOCOL: u8 = 89;
pub const OSPF_VERSION: u8 = 2;

pub const IPV4_PACKET_MTU: usize = 1500;



fn main() {
    println!("Hello, world!");
}
