use core::net;
use std::{collections::HashMap, sync::Arc};

use tokio::{sync::RwLock, task::JoinHandle};

use crate::lsa::{self, Header, Lsa};

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub struct LsaIdentifer {
    pub lsa_type: u32,
    pub link_state_id: u32,
    pub advertising_router: u32,
}

impl LsaIdentifer {
    pub fn from_header(lsa_header: &Header) -> Self {
        Self {
            lsa_type: lsa_header.lsa_type as u32,
            link_state_id: lsa_header.link_state_id,
            advertising_router: lsa_header.advertising_router,
        }
    }
    pub fn to_be_bytes(&self) -> [u8; 12] {
        let mut bytes = [0; 12];
        bytes[0..4].copy_from_slice(&self.lsa_type.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.link_state_id.to_be_bytes());
        bytes[8..12].copy_from_slice(&self.advertising_router.to_be_bytes());
        bytes
    }
    pub fn length() -> usize {
        12
    }
    pub fn try_from_be_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::length() {
            return None;
        }
        let lsa_type = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let link_state_id = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let advertising_router = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        Some(Self {
            lsa_type,
            link_state_id,
            advertising_router,
        })
    }
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
    pub static ref AGE_HANDLES : Arc<RwLock<HashMap<net::Ipv4Addr,Option<JoinHandle<()>>>>> = Arc::new(RwLock::new(HashMap::new()));
}

impl LsaDb {
    pub async fn update_lsdb(&mut self, lsas: Vec<Lsa>) {
        for lsa in lsas {
            match lsa {
                Lsa::Router(rlsa) => {
                    let lsa_id = LsaIdentifer::from_header(&rlsa.header);
                    let mut router_lsa = self.router_lsa.write().await;
                    let arlsa = Arc::new(RwLock::new(rlsa));
                    router_lsa.insert(lsa_id, arlsa);
                }
                Lsa::Network(nlsa) => {
                    let lsa_id = LsaIdentifer::from_header(&nlsa.header);
                    let mut network_lsa = self.network_lsa.write().await;
                    let anlsa = Arc::new(RwLock::new(nlsa));
                    network_lsa.insert(lsa_id, anlsa);
                }
                Lsa::Summary(slsa) => {
                    let lsa_id = LsaIdentifer::from_header(&slsa.header);
                    let mut summary_lsa = self.summary_lsa.write().await;
                    let slsa = Arc::new(RwLock::new(slsa));
                    summary_lsa.insert(lsa_id, slsa);
                }
                Lsa::ASExternal(aslsa) => {
                    let lsa_id = LsaIdentifer::from_header(&aslsa.header);
                    let mut as_external_lsa = self.as_external_lsa.write().await;
                    let aslsa = Arc::new(RwLock::new(aslsa));
                    as_external_lsa.insert(lsa_id, aslsa);
                }
            }
        }
    }
    pub async fn contains_lsa(&self, lsa_id: LsaIdentifer) -> bool {
        let router_lsa = self.router_lsa.read().await;
        if router_lsa.contains_key(&lsa_id) {
            return true;
        }
        drop(router_lsa);
        let network_lsa = self.network_lsa.read().await;
        if network_lsa.contains_key(&lsa_id) {
            return true;
        }
        drop(network_lsa);
        let summary_lsa = self.summary_lsa.read().await;
        if summary_lsa.contains_key(&lsa_id) {
            return true;
        }
        drop(summary_lsa);
        let as_external_lsa = self.as_external_lsa.read().await;
        if as_external_lsa.contains_key(&lsa_id) {
            return true;
        }
        false
    }
    pub fn empty() -> Self {
        Self {
            router_lsa: Arc::new(RwLock::new(HashMap::new())),
            network_lsa: Arc::new(RwLock::new(HashMap::new())),
            summary_lsa: Arc::new(RwLock::new(HashMap::new())),
            as_external_lsa: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub async fn fetch_lsa_headers(&self, inf_trans_delay: u16) -> Vec<Header> {
        let mut headers = Vec::new();
        let router_lsa = self.router_lsa.read().await;
        for (_, rlsa) in router_lsa.iter() {
            let mut locked_rlsa = rlsa.write().await;
            locked_rlsa.header.age += inf_trans_delay;
            headers.push(locked_rlsa.header);
        }
        drop(router_lsa);
        let network_lsa = self.network_lsa.read().await;
        for (_, nlsa) in network_lsa.iter() {
            let mut locked_nlsa = nlsa.write().await;
            locked_nlsa.header.age += inf_trans_delay;
            headers.push(locked_nlsa.header);
        }
        drop(network_lsa);
        let summary_lsa = self.summary_lsa.read().await;
        for (_, slsa) in summary_lsa.iter() {
            let mut locked_slsa = slsa.write().await;
            locked_slsa.header.age += inf_trans_delay;
            headers.push(locked_slsa.header);
        }
        drop(summary_lsa);
        let as_external_lsa = self.as_external_lsa.read().await;
        for (_, aslsa) in as_external_lsa.iter() {
            let mut locked_aslsa = aslsa.write().await;
            locked_aslsa.header.age += inf_trans_delay;
            headers.push(locked_aslsa.header);
        }
        drop(as_external_lsa);
        headers
    }
    pub async fn fetch_lsa(&self, lsa_id: LsaIdentifer) -> Option<Lsa> {
        let router_lsa = self.router_lsa.read().await;
        if let Some(rlsa) = router_lsa.get(&lsa_id) {
            let locked_rlsa = rlsa.read().await;
            return Some(Lsa::Router(locked_rlsa.clone()));
        }
        drop(router_lsa);
        let network_lsa = self.network_lsa.read().await;
        if let Some(nlsa) = network_lsa.get(&lsa_id) {
            let locked_nlsa = nlsa.read().await;
            return Some(Lsa::Network(locked_nlsa.clone()));
        }
        drop(network_lsa);
        let summary_lsa = self.summary_lsa.read().await;
        if let Some(slsa) = summary_lsa.get(&lsa_id) {
            let locked_slsa = slsa.read().await;
            return Some(Lsa::Summary(locked_slsa.clone()));
        }
        drop(summary_lsa);
        let as_external_lsa = self.as_external_lsa.read().await;
        if let Some(aslsa) = as_external_lsa.get(&lsa_id) {
            let locked_aslsa = aslsa.read().await;
            return Some(Lsa::ASExternal(locked_aslsa.clone()));
        }
        None
    }
    pub async fn remove_lsa(&mut self, identifer: LsaIdentifer) {
        let mut router_lsa = self.router_lsa.write().await;
        router_lsa.remove(&identifer);
        drop(router_lsa);
        let mut network_lsa = self.network_lsa.write().await;
        network_lsa.remove(&identifer);
        drop(network_lsa);
        let mut summary_lsa = self.summary_lsa.write().await;
        summary_lsa.remove(&identifer);
        drop(summary_lsa);
        let mut as_external_lsa = self.as_external_lsa.write().await;
        as_external_lsa.remove(&identifer);
    }
}

pub async fn try_remove_lsa(area_id: net::Ipv4Addr, lsa_identifier: LsaIdentifer) {
    let lsdb_map = LSDB_MAP.read().await;
    let lsdb = lsdb_map.get(&area_id).unwrap();
    let mut locked_lsdb = lsdb.write().await;
    locked_lsdb.remove_lsa(lsa_identifier).await;
}

pub async fn fetch_lsa_headers(iaddr: net::Ipv4Addr) -> Vec<Header> {
    let area_id = crate::interface::get_area_id(iaddr).await;
    let lsdb_map = LSDB_MAP.read().await;
    let lsdb = lsdb_map.get(&area_id).unwrap();
    let locked_lsdb = lsdb.read().await;
    let inf_trans_delay = crate::interface::get_inf_trans_delay(iaddr).await;
    locked_lsdb.fetch_lsa_headers(inf_trans_delay as u16).await
}

pub async fn fetch_lsas(
    iaddr: net::Ipv4Addr,
    lsa_identifiers: Vec<LsaIdentifer>,
) -> Option<Vec<Arc<RwLock<Lsa>>>> {
    let area_id = crate::interface::get_area_id(iaddr).await;
    let lsdb_map = LSDB_MAP.read().await;
    let lsdb = lsdb_map.get(&area_id).unwrap();
    let locked_lsdb = lsdb.read().await;
    let mut lsas = Vec::new();
    for lsa_id in lsa_identifiers {
        if !locked_lsdb.contains_lsa(lsa_id).await {
            return None;
        }
        if let Some(lsa) = locked_lsdb.fetch_lsa(lsa_id).await {
            lsas.push(Arc::new(RwLock::new(lsa)));
        } else {
            return None;
        }
    }
    Some(lsas)
}

pub async fn update_lsdb(iaddr: net::Ipv4Addr, lsas: Vec<Lsa>) {
    let area_id = crate::interface::get_area_id(iaddr).await;
    let lsdb_map = LSDB_MAP.read().await;
    let lsdb = lsdb_map.get(&area_id).unwrap();
    let mut locked_lsdb = lsdb.write().await;
    locked_lsdb.update_lsdb(lsas).await;
    // here we should notify the SPF module that the LSDB has been updated
}

pub async fn get_lsdb(area_id: net::Ipv4Addr) -> Arc<RwLock<LsaDb>> {
    let lsdb_map = LSDB_MAP.read().await;
    lsdb_map.get(&area_id).unwrap().clone()
}

pub async fn create_lsa_age_handle(area_id: net::Ipv4Addr) {
    let mut age_handles = AGE_HANDLES.write().await;
    age_handles.insert(area_id, Some(tokio::spawn(age_lsa(area_id))));
}

async fn age_lsa(area_id: net::Ipv4Addr) {
    let age_interval = 1;
    let interval = tokio::time::Duration::from_secs(age_interval);
    loop {
        tokio::time::sleep(interval).await;
        let lsdb_map = LSDB_MAP.read().await;
        let lsdb = lsdb_map.get(&area_id).unwrap();
        let locked_lsdb = lsdb.read().await;
        let router_lsa_list = locked_lsdb.router_lsa.read().await;
        for (_, rlsa) in router_lsa_list.iter() {
            let mut locked_rlsa = rlsa.write().await;
            locked_rlsa.header.age += age_interval as u16;
        }
        drop(router_lsa_list);
        let network_lsa_list = locked_lsdb.network_lsa.read().await;
        for (_, nlsa) in network_lsa_list.iter() {
            let mut locked_nlsa = nlsa.write().await;
            locked_nlsa.header.age += age_interval as u16;
        }
        drop(network_lsa_list);
        let summary_lsa_list = locked_lsdb.summary_lsa.read().await;
        for (_, slsa) in summary_lsa_list.iter() {
            let mut locked_slsa = slsa.write().await;
            locked_slsa.header.age += age_interval as u16;
        }
        drop(summary_lsa_list);
        let as_external_lsa_list = locked_lsdb.as_external_lsa.read().await;
        for (_, aslsa) in as_external_lsa_list.iter() {
            let mut locked_aslsa = aslsa.write().await;
            locked_aslsa.header.age += age_interval as u16;
        }
        drop(as_external_lsa_list);
    }
}
