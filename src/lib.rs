use colored::*;
use std::{
    io::{stdin, stdout, Write},
    net,
};
pub mod area;
pub mod r#as;
pub mod error;
pub mod interface;
pub mod lsa;
pub mod neighbor;
pub mod packet;
pub mod router;
pub mod rtable;

#[allow(non_upper_case_globals)]
pub const AllSPFRouters: net::Ipv4Addr = crate::bits_to_ipv4_addr(0xe0000005);
#[allow(non_upper_case_globals)]
pub const AllDRouters: net::Ipv4Addr = crate::bits_to_ipv4_addr(0xe0000006);

pub const OSPF_VERSION_2: u8 = 2;
pub const OSPF_IP_PROTOCOL_NUMBER: u8 = 89;
pub const MTU: usize = 1500;

pub fn prompt_and_read(prompt: &str) -> String {
    print!("{}", prompt);
    stdout().flush().unwrap();

    let mut input = String::new();
    stdin().read_line(&mut input).expect("read line error");

    input.trim().to_string()
}

pub fn debug(msg: &str) {
    println!("{}", format!("[debug]:{}", msg).yellow());
}

pub fn log(msg: &str) {
    println!("{}", format!("[log]:{}", msg).green());
}

pub fn error(msg: &str) {
    println!("{}", format!("[error]:{}", msg).red());
}

pub const fn bits_to_ipv4_addr(bits: u32) -> net::Ipv4Addr {
    net::Ipv4Addr::new(
        ((bits >> 24) & 0xff) as u8,
        ((bits >> 16) & 0xff) as u8,
        ((bits >> 8) & 0xff) as u8,
        (bits & 0xff) as u8,
    )
}

pub const fn ipv4_addr_to_bits(ip: net::Ipv4Addr) -> u32 {
    (ip.octets()[0] as u32) << 24
        | (ip.octets()[1] as u32) << 16
        | (ip.octets()[2] as u32) << 8
        | ip.octets()[3] as u32
}
