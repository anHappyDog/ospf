#[derive(Clone)]
pub struct RouterLSA {
    pub header: super::Header,
    pub veb: u16,
    pub link_count: u16,
    pub link_states: Vec<LinkState>,
}

pub const ROUTER_LSA_TYPE : u8 = 1;
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
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        if payload.len() < 12 {
            return None;
        }
        let link_id = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
        let link_data = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
        let ls_type = payload[8];
        let tos_count = payload[9];
        let metric = u16::from_be_bytes([payload[10], payload[11]]);
        let mut tos = Vec::new();
        let mut offset = 12;
        for _ in 0..tos_count {
            if offset + 4 > payload.len() {
                return None;
            }
            let tos_value = u32::from_be_bytes([
                payload[offset],
                payload[offset + 1],
                payload[offset + 2],
                payload[offset + 3],
            ]);
            tos.push(tos_value);
            offset += 4;
        }
        Some(Self {
            link_id,
            link_data,
            ls_type,
            tos_count,
            metric,
            tos,
        })
    }
  
}

impl RouterLSA {
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        if payload.len() < super::Header::length() + 4 {
            return None;
        }
        let header = super::Header::try_from_be_bytes(&payload[..super::Header::length()])?;
        let veb = u16::from_be_bytes([
            payload[super::Header::length()],
            payload[super::Header::length() + 1],
        ]);
        let link_count = u16::from_be_bytes([
            payload[super::Header::length() + 2],
            payload[super::Header::length() + 3],
        ]);
        let mut link_states = Vec::new();
        let mut offset = super::Header::length() + 4;
        for _ in 0..link_count {
            if offset + 12 > payload.len() {
                return None;
            }
            let link_state = LinkState::try_from_be_bytes(&payload[offset..])?;
            offset += link_state.length();
            link_states.push(link_state);
        }
        Some(Self {
            header,
            veb,
            link_count,
            link_states,
        })
    }
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
