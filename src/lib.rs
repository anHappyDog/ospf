#![feature(ip_bits)]

use colored::*;
use std::{
    io::{stdin, stdout, Write},
    net,
};
pub mod error;
pub mod area;
pub mod r#as;
pub mod interface;
pub mod lsa;
pub mod neighbor;
pub mod packet;
pub mod router;
pub mod rtable;

#[allow(non_upper_case_globals)]
pub const AllSPFRouters: net::Ipv4Addr = net::Ipv4Addr::new(224, 0, 0, 5);
#[allow(non_upper_case_globals)]
pub const AllDRouters: net::Ipv4Addr = net::Ipv4Addr::new(224, 0, 0, 6);

pub const OSPF_VERSION_2 : u8 = 2;
pub const OSPF_IP_PROTOCOL_NUMBER: u8 = 89;
pub const MTU : usize = 1500;


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
