use core::net;
mod cli;
mod interface;
mod lsa;
mod neighbor;
mod packet;
mod util;
mod err;

lazy_static::lazy_static! {
    pub static ref ROUTER_ID : net::Ipv4Addr = util::input_router_id();
}

pub const ALL_SPF_ROUTERS: net::Ipv4Addr = net::Ipv4Addr::new(224, 0, 0, 5);
pub const ALL_DROTHERS: net::Ipv4Addr = net::Ipv4Addr::new(224, 0, 0, 6);

pub const OSPF_IP_PROTOCOL: u8 = 89;
pub const OSPF_VERSION: u8 = 2;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    interface::init().await?;
    cli::cli().await?;
    Ok(())
}
