use ospf_lib::interface;
use ospf_lib::prompt_and_read;
use std::net;
use std::sync::Arc;
use std::sync::Mutex;

mod cli;

/*
 * What does the function main do?
 * - init the router and interface list
 * - open the ospf-cli and wait for the user input
 */

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ospf_lib::init();
    cli::cli(ospf_lib::ROUTER_ID.clone())
}
