use clap::Arg;
use clap::Subcommand;
use ospf_lib::interface;
use ospf_lib::prompt_and_read;
use ospf_lib::router;
use pnet::datalink::Channel::Ethernet;
use pnet::datalink::{self, NetworkInterface};
use pnet::packet::dns::DnsTypes::A;
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::Packet;
use std::io::stdin;
use std::net;
use std::sync::Arc;
use std::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router_id = prompt_and_read("please enter router id:")
        .parse::<net::Ipv4Addr>()
        .unwrap();
    let router = Arc::new(Mutex::new(router::Router::new(router_id)));
    let interfaces =
        interface::create_interfaces(router.clone()).expect("No interface found in the machine.");
    // let mut router = router::create_simulated_router(interfaces);
    router.lock().unwrap().add_interfaces(interfaces);
    Ok(())
}
