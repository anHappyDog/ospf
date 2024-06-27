use std::net;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

lazy_static::lazy_static! {
    // THE KEY IS THE AREA ID
    pub static ref AREA_MAP: Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Area>>>>> = Arc::new(RwLock::new(HashMap::new()));
    // REMEMBER THE LSDB IN THE AREA.
    pub static ref LSDB_MAP: Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<u32>>>>> = Arc::new(RwLock::new(HashMap::new()));
    // THE AREA'S CURRENT DR
    pub static ref DR_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,net::Ipv4Addr>>> = Arc::new(RwLock::new(HashMap::new()));
    // THE AREA'S CURRENT BDR
    pub static ref BDR_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,net::Ipv4Addr>>> = Arc::new(RwLock::new(HashMap::new()));

}

pub struct Area {
    pub id: net::Ipv4Addr,
    pub addr_range_list: Vec<AddrRange>,
    pub advertise_or_not: bool,
    pub external_routing_capability: bool,
}

pub struct AddrRange {
    pub addr: net::Ipv4Addr,
    pub mask: net::Ipv4Addr,
}

pub async fn add(area_id: net::Ipv4Addr) {
    let mut area_map = AREA_MAP.write().await;
    area_map.insert(
        area_id,
        Arc::new(RwLock::new(Area {
            id: area_id,
            addr_range_list: Vec::new(),
            advertise_or_not: true,
            external_routing_capability: true,
        })),
    );
    drop(area_map);
    let mut lsdb_map = LSDB_MAP.write().await;
    lsdb_map.insert(area_id, Arc::new(RwLock::new(0)));
    drop(lsdb_map);
    let mut dr_map = DR_MAP.write().await;
    dr_map.insert(area_id, net::Ipv4Addr::new(0, 0, 0, 0));
    drop(dr_map);
    let mut bdr_map = BDR_MAP.write().await;
    bdr_map.insert(area_id, net::Ipv4Addr::new(0, 0, 0, 0));
    drop(bdr_map);
    crate::util::debug(&format!("area_id : {} added.", area_id));
}

pub async fn list() {
    let area_map = AREA_MAP.read().await;
    let dr_map = DR_MAP.read().await;
    let bdr_map = BDR_MAP.read().await;
    for (area_id, area) in area_map.iter() {
        println!("---------------------");
        println!("area_id :{}", area_id);
        println!("advertise_or_not :{}", area.read().await.advertise_or_not);
        println!(
            "external_routing_capability :{}",
            area.read().await.external_routing_capability
        );
        println!(
            "current designated router : {}",
            dr_map.get(area_id).unwrap()
        );
        println!(
            "current backup designated router : {}",
            bdr_map.get(area_id).unwrap()
        );
    }
}

pub async fn display(area_id: net::Ipv4Addr) {
    let area_map = AREA_MAP.read().await;
    let dr_map = DR_MAP.read().await;
    let bdr_map = BDR_MAP.read().await;
    if let None = area_map.get(&area_id) {
        crate::util::error(&format!("area_id : {} not found", area_id));
        return;
    }
    println!("---------------------");
    println!("area_id :{}", area_id);
    println!(
        "current designated router : {}",
        dr_map.get(&area_id).unwrap()
    );
    println!(
        "current backup designated router : {}",
        bdr_map.get(&area_id).unwrap()
    );
}
