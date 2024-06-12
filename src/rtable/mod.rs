pub mod entry;

pub struct RouteTable {
    entries: Vec<entry::RouteTableEntry>,
}

impl RouteTable {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}
