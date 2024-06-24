use colored::*;
use std::{io, io::Write, net};

pub fn prompt_and_read(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("read line error");
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

pub fn input_router_id() -> net::Ipv4Addr {
    loop {
        match prompt_and_read("please enter router id:").parse::<net::Ipv4Addr>() {
            Ok(id) => {
                return id;
            }
            Err(_) => {
                println!("Invalid router id, please try again.");
            }
        }
    }
}
