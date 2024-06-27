pub mod handle;
pub mod status;


// the key is the neighbors ipv4 address
lazy_static::lazy_static! {
    pub static ref NEIGHBOR_STATUS_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<status::Status>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref NEIGHBOR_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Neighbor>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref NEIHGBOR_LSA_RETRANS_LIST_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Vec<crate::lsa::Lsa>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref NEIGHBOR_SUMMARY_LIST_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Vec<crate::lsa::Lsa>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref NEIGHBOR_LSR_LIST_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Vec<crate::packet::lsr::Lsr>>>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref NEIGHBOR_LAST_DD_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,crate::packet::dd::Dd>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub struct Neighbor {
    pub state: status::Status,
    pub master: bool,
    pub dd_seq: u32,
    pub id: net::Ipv4Addr,
    pub priority: u8,
    pub ipv4_addr: net::Ipv4Addr,
    pub options: u8,
    pub dr: net::Ipv4Addr,
    pub bdr: net::Ipv4Addr,
}
