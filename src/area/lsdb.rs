use core::net;
use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::lsa;

#[derive(Clone, Copy)]
pub struct LsaIdentifer {
    pub lsa_type: u8,
    pub link_state_id: u32,
    pub advertising_router: u32,
}

pub struct LsaDb {
    pub router_lsa: Arc<RwLock<HashMap<LsaIdentifer, Arc<RwLock<lsa::router::RouterLSA>>>>>,
    pub network_lsa: Arc<RwLock<HashMap<LsaIdentifer, Arc<RwLock<lsa::network::NetworkLSA>>>>>,
    pub summary_lsa: Arc<RwLock<HashMap<LsaIdentifer, Arc<RwLock<lsa::summary::SummaryLSA>>>>>,
    pub as_external_lsa:
        Arc<RwLock<HashMap<LsaIdentifer, Arc<RwLock<lsa::as_external::ASExternalLSA>>>>>,
}


lazy_static::lazy_static! {
    pub static ref LSDB_MAP: Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<LsaDb>>>>> = Arc::new(RwLock::new(HashMap::new()));
}


impl LsaDb {
    pub fn empty() ->Self {
        Self {
            router_lsa: Arc::new(RwLock::new(HashMap::new())),
            network_lsa: Arc::new(RwLock::new(HashMap::new())),
            summary_lsa: Arc::new(RwLock::new(HashMap::new())),
            as_external_lsa: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}


pub async fn fetch_lsa_headers(area_id : net::Ipv4Addr) -> Vec<lsa::Header> {
    let mut  headers = Vec::new();
    let lsdb_map = LSDB_MAP.read().await;
    let lsdb = lsdb_map.get(&area_id).unwrap();
    let locked_lsdb = lsdb.read().await;
    // for (lsa)
    // for (lsa_id, _) in &locked_lsdb.router_lsa.read().await {
    //     headers.push(lsa::Header {
    //         lsa_type: lsa_id.lsa_type,
    //         link_state_id: lsa_id.link_state_id,
    //         advertising_router: lsa_id.advertising_router,
    //     });
    // }

    headers
}

