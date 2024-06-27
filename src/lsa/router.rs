#[derive(Clone)]
pub struct RouterLSA {
    pub header: super::Header,
    pub veb: u16,
    pub link_count: u16,
    pub link_states: Vec<LinkState>,
}

#[derive(Clone)]
pub struct LinkState {
    pub link_id: u32,
    pub link_data: u32,
    pub ls_type: u8,
    pub tos_count: u8,
    pub metric: u16,
    pub tos: Vec<u32>,
}

impl LinkState {
    // the pass tos is host endian
    pub fn tos_type(tos: u32) -> u8 {
        (tos >> 24) as u8
    }
    pub fn tos_metric(tos: u32) -> u16 {
        (tos & 0x00ffffff) as u16
    }
    pub fn length(&self) -> usize {
        12 + self.tos.len() * 4
    }
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.link_id.to_be_bytes());
        bytes.extend_from_slice(&self.link_data.to_be_bytes());
        bytes.extend_from_slice(&self.ls_type.to_be_bytes());
        bytes.extend_from_slice(&self.tos_count.to_be_bytes());
        bytes.extend_from_slice(&self.metric.to_be_bytes());
        for tos in &self.tos {
            bytes.extend_from_slice(&tos.to_be_bytes());
        }
        bytes
    }
}

impl RouterLSA {
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_be_bytes());
        bytes.extend_from_slice(&self.veb.to_be_bytes());
        bytes.extend_from_slice(&self.link_count.to_be_bytes());
        for link_state in &self.link_states {
            bytes.extend_from_slice(&link_state.to_be_bytes());
        }
        bytes
    }
    pub fn length(&self) -> usize {
        let mut length = super::Header::length() + 4;
        for link_state in &self.link_states {
            length += link_state.length();
        }
        length
    }
}
