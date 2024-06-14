use pnet::datalink;

use crate::lsa::network::NetworkLinkStateAdvertisement;
use crate::lsa::LinkStateAdvertisement;
use crate::rtable;
use crate::{debug, interface};
use std::collections::HashMap;
use std::net;

pub struct Router<'a> {
    router_table: Vec<rtable::RouteTable>,
    router_id: net::Ipv4Addr,
    interfaces: HashMap<String, interface::Interface<'a>>,
}

pub fn create_simulated_router(interfaces: Vec<interface::Interface>) -> Router {
    let router_id: u32;
    println!("Creating a simulated router...");
    loop {
        let router_id_str = crate::prompt_and_read("Please enter the router id(a 32-bit number): ");
        match router_id_str.parse::<u32>() {
            Ok(id) => {
                router_id = id;
                break;
            }
            Err(_) => {
                println!("Invalid router id, please try again.");
            }
        }
    }
    let mut router = Router::new(net::Ipv4Addr::from_bits(router_id));
    router.add_interfaces(interfaces);
    println!("----------------------------------------------------");
    println!("Router [{}]created successfully.", router.router_id);
    println!("----------------------------------------------------");
    println!("Router interfaces: ");
    for (name, interface) in router.interfaces.iter() {
        println!(
            "{}: {}/{}, belongs to {}",
            name, interface.ip_addr, interface.network_mask, interface.aread_id
        );
    }
    println!("----------------------------------------------------");
    router
}

impl<'a> Router<'a> {
    pub async fn init(&'a mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "Router [{}]start working,now init its interfaces...",
            self.router_id
        );
        for (name, interface) in self.interfaces.iter_mut() {
            debug(&format!("init interface [{}]", name));
            interface.init_handlers().await?;
            debug(&format!("interface [{}] init successfully", name));
        }
        Ok(())
    }

    pub fn get_router_id(&self) -> net::Ipv4Addr {
        self.router_id
    }
    pub fn new(router_id: net::Ipv4Addr) -> Self {
        Self {
            router_table: Vec::new(),
            interfaces: HashMap::new(),
            router_id,
        }
    }
    pub fn add_interface(&mut self, interface: interface::Interface<'a>) {
        self.interfaces.insert(interface.name.clone(), interface);
    }
    pub fn add_interfaces(&mut self, interfaces: Vec<interface::Interface<'a>>) {
        for interface in interfaces {
            self.add_interface(interface);
        }
    }
}
