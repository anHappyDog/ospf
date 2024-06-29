use std::{collections::HashMap, net, sync::Arc};

use status::Status;
use tokio::sync::RwLock;

use crate::{
    area::{self, lsdb::LsaIdentifer},
    interface,
    packet::{dd::DD, hello::Hello},
};

pub mod event;
pub mod handle;
pub mod status;

// this key is the interface ipv4addr
// the inner ket is the neighbors **ipv4 addr**
// can get the id by the ospf packet 's inner function `get_neighbor_addr`
lazy_static::lazy_static! {
    pub static ref NEIGHBOR_STATUS_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<status::Status>>>>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref NEIHGBOR_LSA_RETRANS_LIST_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Vec<area::lsdb::LsaIdentifer>>>>>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref NEIGHBOR_SUMMARY_LIST_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Vec<area::lsdb::LsaIdentifer>>>>>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref NEIGHBOR_LSR_LIST_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Vec<area::lsdb::LsaIdentifer>>>>>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref NEIGHBOR_LAST_DD_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<HashMap<net::Ipv4Addr,crate::packet::dd::DD>>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref INT_NEIGHBORS_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Neighbor>>>>>>>> = Arc::new(RwLock::new(HashMap::new()));
}

#[derive(Clone, Copy)]
pub struct Neighbor {
    pub master: bool,
    pub dd_seq: u32,
    pub id: net::Ipv4Addr,
    pub priority: u8,
    pub ipv4_addr: net::Ipv4Addr,
    pub options: u8,
    pub dr: net::Ipv4Addr,
    pub bdr: net::Ipv4Addr,
}

impl Neighbor {
    pub fn from_hello_packet(hello: &Hello, naddr: net::Ipv4Addr, nid: net::Ipv4Addr) -> Self {
        Self {
            master: false,
            dd_seq: 0,
            id: nid,
            priority: hello.router_priority,
            ipv4_addr: naddr,
            options: hello.options,
            dr: hello.designated_router.into(),
            bdr: hello.backup_designated_router.into(),
        }
    }
}

pub async fn get_int_neighbors(
    iaddr: net::Ipv4Addr,
) -> Arc<RwLock<HashMap<net::Ipv4Addr, Arc<RwLock<Neighbor>>>>> {
    let int_neighbors = INT_NEIGHBORS_MAP.read().await;
    int_neighbors.get(&iaddr).unwrap().clone()
}

pub async fn get_naddr_by_id(
    iaddr: net::Ipv4Addr,
    neighbor_id: net::Ipv4Addr,
) -> Option<net::Ipv4Addr> {
    let int_neighbors = INT_NEIGHBORS_MAP.read().await;
    let neighbors = int_neighbors.get(&iaddr).unwrap();
    let locked_neighbors = neighbors.read().await;
    for (naddr, neighbor) in locked_neighbors.iter() {
        let locked_neighbor = neighbor.read().await;
        if locked_neighbor.id == neighbor_id {
            return Some(naddr.clone());
        }
    }
    return None;
}

pub async fn get_status_by_id(iaddr: net::Ipv4Addr, nid: net::Ipv4Addr) -> Option<Status> {
    let int_neighbors = INT_NEIGHBORS_MAP.read().await;
    let neighbors = int_neighbors.get(&iaddr).unwrap();
    let locked_neighbors = neighbors.read().await;
    for (naddr, neighbor) in locked_neighbors.iter() {
        let locked_neighbor = neighbor.read().await;
        if locked_neighbor.id == nid {
            return Some(get_status(iaddr, naddr.clone()).await);
        }
    }
    return None;
}

pub async fn update_neighbor(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr, packet: &Hello) {
    let g_neighbors = INT_NEIGHBORS_MAP.read().await;
    let int_neighbors = g_neighbors.get(&iaddr).unwrap();
    let locked_neighbors = int_neighbors.read().await;
    let neighbor = locked_neighbors.get(&naddr).unwrap();
    let mut locked_neighbor = neighbor.write().await;

    if packet.router_priority != locked_neighbor.priority {
        locked_neighbor.priority = packet.router_priority;
        tokio::spawn(interface::event::send(
            iaddr,
            interface::event::Event::NeighborChange(naddr),
        ));
    }
    if packet.designated_router == locked_neighbor.id.into()
        && packet.backup_designated_router == 0
        && interface::get_status(iaddr).await == interface::status::Status::Waiting
    {
        tokio::spawn(interface::event::send(
            iaddr,
            interface::event::Event::BackupSeen,
        ));
    } else if packet.designated_router == locked_neighbor.id.into()
        && packet.designated_router != locked_neighbor.id.into()
    {
        tokio::spawn(interface::event::send(
            iaddr,
            interface::event::Event::NeighborChange(naddr),
        ));
    } else if packet.designated_router != locked_neighbor.id.into()
        && packet.designated_router == locked_neighbor.id.into()
    {
        tokio::spawn(interface::event::send(
            iaddr,
            interface::event::Event::NeighborChange(naddr),
        ));
    }

    if packet.backup_designated_router == locked_neighbor.ipv4_addr.into()
        && interface::get_status(iaddr).await == interface::status::Status::Waiting
    {
        tokio::spawn(interface::event::send(
            iaddr,
            interface::event::Event::BackupSeen,
        ));
    } else if packet.backup_designated_router == locked_neighbor.ipv4_addr.into()
        && locked_neighbor.bdr != locked_neighbor.ipv4_addr
    {
        tokio::spawn(interface::event::send(
            iaddr,
            interface::event::Event::NeighborChange(naddr),
        ));
    } else if packet.backup_designated_router != locked_neighbor.ipv4_addr.into()
        && locked_neighbor.bdr == locked_neighbor.ipv4_addr
    {
        tokio::spawn(interface::event::send(
            iaddr,
            interface::event::Event::NeighborChange(naddr),
        ));
    }

    // now update the neighbor
    locked_neighbor.id = packet.header.router_id;
    locked_neighbor.options = packet.options;
    locked_neighbor.dr = packet.designated_router.into();
    locked_neighbor.bdr = packet.backup_designated_router.into();
}

pub async fn contains_neighbor(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) -> bool {
    let int_neighbors = INT_NEIGHBORS_MAP.read().await;
    let neighbors = int_neighbors.get(&iaddr).unwrap();
    let locked_neighbors = neighbors.read().await;
    locked_neighbors.contains_key(&naddr)
}

pub async fn get_status(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) -> status::Status {
    let int_neighbor_status = NEIGHBOR_STATUS_MAP.read().await;
    let neighbor_status = int_neighbor_status.get(&iaddr).unwrap();
    let locked_neighbor_status = neighbor_status.read().await;
    let status = locked_neighbor_status.get(&naddr).unwrap();
    let locked_status = status.read().await;
    *locked_status
}

pub async fn get_ddseqno(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) -> u32 {
    let int_neighbors = INT_NEIGHBORS_MAP.read().await;
    let neighbors = int_neighbors.get(&iaddr).unwrap();
    let locked_neighbors = neighbors.read().await;
    let neighbor = locked_neighbors.get(&naddr).unwrap();
    let locked_neighbor = neighbor.read().await;
    locked_neighbor.dd_seq
}

pub async fn set_option(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr, option: u8) {
    let int_neighbors = INT_NEIGHBORS_MAP.read().await;
    let neighbors = int_neighbors.get(&iaddr).unwrap();
    let locked_neighbors = neighbors.read().await;
    let neighbor = locked_neighbors.get(&naddr).unwrap();
    let mut locked_neighbor = neighbor.write().await;
    locked_neighbor.options = option;
}

pub async fn set_ddseqno(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr, seq: u32) {
    let int_neighbors = INT_NEIGHBORS_MAP.read().await;
    let neighbors = int_neighbors.get(&iaddr).unwrap();
    let locked_neighbors = neighbors.read().await;
    let neighbor = locked_neighbors.get(&naddr).unwrap();
    let mut locked_neighbor = neighbor.write().await;
    locked_neighbor.dd_seq = seq;
}

pub async fn save_last_dd(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr, dd: crate::packet::dd::DD) {
    let last_dd_map = NEIGHBOR_LAST_DD_MAP.read().await;
    let last_dd = last_dd_map.get(&iaddr).unwrap();
    let mut locked_last_dd = last_dd.write().await;
    locked_last_dd.insert(naddr, dd);
}

pub async fn update_lsr_list(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr, dd: DD) {
    let lsr_list = NEIGHBOR_LSR_LIST_MAP.read().await;
    let lsr = lsr_list.get(&iaddr).unwrap();
    let locked_lsr = lsr.read().await;
    let lsr_list = locked_lsr.get(&naddr).unwrap();
    let mut locked_lsr_list = lsr_list.write().await;
    let area_id = interface::get_area_id(iaddr).await;
    let lsdb = area::lsdb::get_lsdb(area_id).await;
    let locked_lsdb = lsdb.read().await;

    for lsa_header in dd.lsa_headers.iter() {
        let identifier = LsaIdentifer {
            lsa_type: lsa_header.lsa_type as u32,
            link_state_id: lsa_header.link_state_id,
            advertising_router: lsa_header.advertising_router,
        };
        if !locked_lsdb.contains_lsa(identifier).await {
            locked_lsr_list.push(identifier);
        }
    }
}

// when a lsa header is in a lsack packet, then use this to
/// remove the lsa identifier from the neighbor's retrans list.
pub async fn ack_lsa(
    iaddr: net::Ipv4Addr,
    naddr: net::Ipv4Addr,
    lsa_identifer: area::lsdb::LsaIdentifer,
) {
    let g_retrans_list = NEIHGBOR_LSA_RETRANS_LIST_MAP.read().await;
    let retrans_list = g_retrans_list.get(&iaddr).unwrap();
    let locked_retrans_list = retrans_list.read().await;
    let n_retrans_list = locked_retrans_list.get(&naddr).unwrap();
    let mut locked_n_retrans_list = n_retrans_list.write().await;
    locked_n_retrans_list.retain(|x| x != &lsa_identifer);
}

pub async fn get_trans_list(
    iaddr: net::Ipv4Addr,
    naddr: net::Ipv4Addr,
) -> Arc<RwLock<Vec<area::lsdb::LsaIdentifer>>> {
    let g_lsa_retrans_list = NEIHGBOR_LSA_RETRANS_LIST_MAP.read().await;
    let lsa_retrans_list = g_lsa_retrans_list.get(&iaddr).unwrap();
    let locked_lsa_retrans_list = lsa_retrans_list.read().await;
    let n_lsa_retrans_list = locked_lsa_retrans_list.get(&naddr).unwrap();
    n_lsa_retrans_list.clone()
}

pub async fn fill_retrans_list(
    iaddr: net::Ipv4Addr,
    naddr: net::Ipv4Addr,
    lsa_identifers: Vec<area::lsdb::LsaIdentifer>,
) {
    let g_lsa_retrans_list = NEIHGBOR_LSA_RETRANS_LIST_MAP.read().await;
    let lsa_retrans_list = g_lsa_retrans_list.get(&iaddr).unwrap();
    let locked_lsa_retrans_list = lsa_retrans_list.read().await;
    let n_lsa_retrans_list = locked_lsa_retrans_list.get(&naddr).unwrap();
    let mut locked_n_lsa_retrans_list = n_lsa_retrans_list.write().await;
    locked_n_lsa_retrans_list.extend(lsa_identifers);
}

pub async fn is_lsr_list_empty(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) -> bool {
    let lsr_list = NEIGHBOR_LSR_LIST_MAP.read().await;
    let lsr = lsr_list.get(&iaddr).unwrap();
    let locked_lsr = lsr.read().await;
    let lsr = locked_lsr.get(&naddr).unwrap();
    let locked_lsr = lsr.read().await;
    locked_lsr.is_empty()
}

pub async fn clear_lsr_list(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) {
    let lsr_list = NEIGHBOR_LSR_LIST_MAP.read().await;
    let lsr = lsr_list.get(&iaddr).unwrap();
    let locked_lsr = lsr.read().await;
    let lsr = locked_lsr.get(&naddr).unwrap();
    let mut locked_lsr = lsr.write().await;
    locked_lsr.clear();
}

pub async fn clear_summary_list(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) {
    let summary_list = NEIGHBOR_SUMMARY_LIST_MAP.read().await;
    let summary = summary_list.get(&iaddr).unwrap();
    let locked_summary = summary.read().await;
    let summary = locked_summary.get(&naddr).unwrap();
    let mut locked_summary = summary.write().await;
    locked_summary.clear();
}

pub async fn clear_lsa_retrans_list(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) {
    let lsa_retrans_list = NEIHGBOR_LSA_RETRANS_LIST_MAP.read().await;
    let lsa_retrans = lsa_retrans_list.get(&iaddr).unwrap();
    let locked_lsa_retrans = lsa_retrans.read().await;
    let lsa_retrans = locked_lsa_retrans.get(&naddr).unwrap();
    let mut locked_lsa_retrans = lsa_retrans.write().await;
    locked_lsa_retrans.clear();
}

pub async fn is_duplicated_dd(
    iaddr: net::Ipv4Addr,
    naddr: net::Ipv4Addr,
    dd: &crate::packet::dd::DD,
) -> bool {
    let last_dd_map = NEIGHBOR_LAST_DD_MAP.read().await;
    let last_dd = last_dd_map.get(&iaddr).unwrap();
    let locked_last_dd = last_dd.read().await;
    match locked_last_dd.get(&naddr) {
        Some(last_dd) => last_dd.dd_sequence_number == dd.dd_sequence_number,
        None => false,
    }
}

pub async fn is_master(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) -> bool {
    let int_neighbors = INT_NEIGHBORS_MAP.read().await;
    let neighbors = int_neighbors.get(&iaddr).unwrap();
    let locked_neighbors = neighbors.read().await;
    let neighbor = locked_neighbors.get(&naddr).unwrap();
    let locked_neighbor = neighbor.read().await;
    locked_neighbor.master
}

pub async fn get_options(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) -> u8 {
    let int_neighbors = INT_NEIGHBORS_MAP.read().await;
    let neighbors = int_neighbors.get(&iaddr).unwrap();
    let locked_neighbors = neighbors.read().await;
    let neighbor = locked_neighbors.get(&naddr).unwrap();
    let locked_neighbor = neighbor.read().await;
    locked_neighbor.options
}

pub async fn set_master(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr, master: bool) {
    let int_neighbors = INT_NEIGHBORS_MAP.read().await;
    let neighbors = int_neighbors.get(&iaddr).unwrap();
    let locked_neighbors = neighbors.read().await;
    let neighbor = locked_neighbors.get(&naddr).unwrap();
    let mut locked_neighbor = neighbor.write().await;
    locked_neighbor.master = master;
}

pub async fn set_status(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr, new_status: status::Status) {
    let int_neighbor_status = NEIGHBOR_STATUS_MAP.read().await;
    let neighbor_status = int_neighbor_status.get(&iaddr).unwrap();
    let mut locked_neighbor_status = neighbor_status.write().await;
    locked_neighbor_status.insert(naddr, Arc::new(RwLock::new(new_status)));
}

pub async fn add(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr, neighbor: Neighbor) {
    let int_neighbors = INT_NEIGHBORS_MAP.read().await;
    let neighbors = int_neighbors.get(&iaddr).unwrap();
    let mut locked_neighbors = neighbors.write().await;
    locked_neighbors.insert(naddr, Arc::new(RwLock::new(neighbor)));
    drop(locked_neighbors);
    drop(int_neighbors);

    let neighbor_status = NEIGHBOR_STATUS_MAP.read().await;
    let mut locked_neighbor_status = neighbor_status.get(&iaddr).unwrap().write().await;
    locked_neighbor_status.insert(naddr, Arc::new(RwLock::new(status::Status::Down)));
    drop(locked_neighbor_status);
    drop(neighbor_status);

    let lsa_retrans_list = NEIHGBOR_LSA_RETRANS_LIST_MAP.read().await;
    let mut locked_lsa_retrans_list = lsa_retrans_list.get(&iaddr).unwrap().write().await;
    locked_lsa_retrans_list.insert(naddr, Arc::new(RwLock::new(Vec::new())));
    drop(locked_lsa_retrans_list);
    drop(lsa_retrans_list);

    let summary_list = NEIGHBOR_SUMMARY_LIST_MAP.read().await;
    let mut locked_summary_list = summary_list.get(&iaddr).unwrap().write().await;
    locked_summary_list.insert(naddr, Arc::new(RwLock::new(Vec::new())));
    drop(locked_summary_list);
    drop(summary_list);

    let lsr_list = NEIGHBOR_LSR_LIST_MAP.read().await;
    let mut locked_lsr_list = lsr_list.get(&iaddr).unwrap().write().await;
    locked_lsr_list.insert(naddr, Arc::new(RwLock::new(Vec::new())));
    drop(locked_lsr_list);
    drop(lsr_list);

    let handle_list = handle::HANDLE_MAP.read().await;
    let n_handle_list = handle_list.get(&iaddr).unwrap();
    let mut locked_handle_list = n_handle_list.write().await;
    locked_handle_list.insert(naddr, handle::Handle::new(naddr, iaddr));
}

pub async fn is_adjacent(iaddr: net::Ipv4Addr, naddr: net::Ipv4Addr) -> bool {
    let network_type = interface::get_network_type(iaddr).await;
    match network_type {
        interface::NetworkType::PointToPoint
        | interface::NetworkType::PointToMultipoint
        | interface::NetworkType::VirtualLink => return true,
        _ => {
            let dr_id = interface::get_dr(iaddr).await;
            let bdr_id = interface::get_bdr(iaddr).await;
            let router_id = crate::ROUTER_ID.clone();
            if dr_id == router_id || bdr_id == router_id {
                return true;
            }
            let neighbors = get_int_neighbors(iaddr).await;
            let locked_neighbors = neighbors.read().await;
            let neighbor = locked_neighbors.get(&naddr).unwrap();
            let locked_neighbor = neighbor.read().await;
            if locked_neighbor.id == dr_id || locked_neighbor.id == bdr_id {
                return true;
            }
        }
    }
    false
}

pub async fn init(iaddrs: Vec<net::Ipv4Addr>) {
    let mut summary_list = NEIGHBOR_SUMMARY_LIST_MAP.write().await;
    let mut lsa_retrans_list = NEIHGBOR_LSA_RETRANS_LIST_MAP.write().await;
    let mut neighbor_status = NEIGHBOR_STATUS_MAP.write().await;
    let mut int_neighbors = INT_NEIGHBORS_MAP.write().await;
    let mut lsr_list = NEIGHBOR_LSR_LIST_MAP.write().await;
    let mut last_dd_map = NEIGHBOR_LAST_DD_MAP.write().await;

    for iaddr in iaddrs {
        int_neighbors.insert(iaddr, Arc::new(RwLock::new(HashMap::new())));

        neighbor_status.insert(iaddr, Arc::new(RwLock::new(HashMap::new())));

        lsa_retrans_list.insert(iaddr, Arc::new(RwLock::new(HashMap::new())));

        summary_list.insert(iaddr, Arc::new(RwLock::new(HashMap::new())));

        lsr_list.insert(iaddr, Arc::new(RwLock::new(HashMap::new())));

        last_dd_map.insert(iaddr, Arc::new(RwLock::new(HashMap::new())));
    }
}
