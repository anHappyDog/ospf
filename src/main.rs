use pnet::datalink::Channel::Ethernet;
use pnet::datalink::{self, NetworkInterface};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::Packet;

use std::net;

fn test1() {
    let interface_name = "eth0"; // 根据你的系统环境调整网卡名称
    let interface = datalink::interfaces()
        .into_iter()
        .find(|iface: &NetworkInterface| iface.name == interface_name)
        .expect("Error getting interface");

    let (_, mut rx) = match datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!("Error creating datalink channel: {}", e),
    };

    loop {
        match rx.next() {
            Ok(packet) => {
                // 这里你可以处理每一个收到的 IP 包
                println!("Received packet!,{:#?}", packet);
            }
            Err(e) => {
                println!("An error occurred while reading: {}", e);
            }
        }
    }
}

fn test2() {
    let interfaces = datalink::interfaces();
    for interface in interfaces {
        println!("Name: {}", interface.name);
        println!("Description: {:?}", interface.description);

        for ip in interface.ips {
            println!("IP Address: {}", ip.ip());
            println!("Network Mask: {:?}", ip.mask());
        }

        println!("-----------------------------------");
    }
}

fn main() {
    println!("init ospf process...");
    let router = ospf_lib::router::Router::new(net::Ipv4Addr::new(0, 0, 0, 0), net::Ipv4Addr::new(0, 0, 0, 0));
    
    test2();
}
