

lazy_static::lazy_static! {
    pub static ref HANDLE_MAP : Arc<RwLock<HashMap<net::Ipv4Addr,Arc<RwLock<Handle>>>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub struct Handle {}