pub mod lsdb;
use std::net;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

lazy_static::lazy_static! {
    // THE KEY IS THE AREA ID
    pub static ref AREA_MAP: Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Area>>>>> = Arc::new(RwLock::new(HashMap::new()));
    
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



pub async fn exists(area_id: net::Ipv4Addr) -> bool {
    let area_map = AREA_MAP.read().await;
    let result = area_map.get(&area_id);
    result.is_some()
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

    let mut lsdb_map = lsdb::LSDB_MAP.write().await;
    lsdb_map.insert(area_id, Arc::new(RwLock::new(lsdb::LsaDb::empty())));
    drop(lsdb_map);

    lsdb::create_lsa_age_handle(area_id).await;
    crate::util::debug(&format!("area_id : {} added.", area_id));
}

pub async fn list() {
    let area_map = AREA_MAP.read().await;
    for (area_id, area) in area_map.iter() {
        println!("---------------------");
        println!("area_id :{}", area_id);
        println!("advertise_or_not :{}", area.read().await.advertise_or_not);
        println!(
            "external_routing_capability :{}",
            area.read().await.external_routing_capability
        );
    }
}

pub async fn display(area_id: net::Ipv4Addr) {
    let area_map = AREA_MAP.read().await;
    if let None = area_map.get(&area_id) {
        crate::util::error(&format!("area_id : {} not found", area_id));
        return;
    }
    println!("---------------------");
    println!("area_id :{}", area_id);

}
