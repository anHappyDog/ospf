
use crate::rtable;
use crate::{debug, interface};
use std::collections::HashMap;
use std::net;
use std::sync::{Arc, Mutex};

pub struct Router {
    router_table: Vec<rtable::RouteTable>,
    router_id: net::Ipv4Addr,
    interfaces: HashMap<String, Arc<Mutex<interface::Interface>>>,
}

pub fn create_simulated_router(
    interfaces: HashMap<String, Arc<Mutex<interface::Interface>>>,
) -> Arc<Mutex<Router>> {
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
    let router = Arc::new(Mutex::new(Router::new(net::Ipv4Addr::from_bits(router_id))));
    for (_, interface) in &interfaces {
        let mut interface = interface.lock().unwrap();
        interface.router = router.clone();
    }
    let mut locked_router = router.lock().unwrap();
    locked_router.add_interfaces(interfaces);
    drop(locked_router);
    router
}

impl Router {
    pub const MAX_INNER_PACKET_QUEUE_SIZE: usize = 100;
    pub fn get_router_id(&self) -> net::Ipv4Addr {
        self.router_id
    }
    pub async fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        debug("Router initialized.");
        for (_, interface) in &self.interfaces {
            let mut interface = interface.lock().unwrap();
            interface.init_handlers().await?;
        }
        Ok(())
    }
    pub fn new(router_id: net::Ipv4Addr) -> Self {
        Self {
            router_table: Vec::new(),
            interfaces: HashMap::new(),
            router_id,
        }
    }
    pub fn add_interface(&mut self, name: String, interface: Arc<Mutex<interface::Interface>>) {
        self.interfaces.insert(name, interface);
    }
    pub fn add_interfaces(
        &mut self,
        interfaces: HashMap<String, Arc<Mutex<interface::Interface>>>,
    ) {
        self.interfaces = interfaces;
    }
}
