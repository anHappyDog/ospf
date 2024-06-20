use ospf_lib::interface;
use ospf_lib::prompt_and_read;
use ospf_lib::router;
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

    let _ = router.lock().unwrap().init().await;
    loop {
        
    }
    Ok(())
}
