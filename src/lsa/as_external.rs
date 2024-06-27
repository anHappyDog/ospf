use super::Header;

#[derive(Clone)]
pub struct ASExternalLSA {
    pub header: Header,
    pub network_mask : u32,
    pub e_bits : u8,
    pub metric : [u8;3],
    pub forwarding_address : u32,
    pub external_route_tag : u32,
    pub tos : Vec<TosDes>
}

#[derive(Clone, Copy)]
pub struct TosDes {
    pub tos_ebits : u8,
    pub tos_metric : [u8;3],
    pub tos_forwarding_address : u32,
    pub tos_external_route_tag : u32,
}

impl TosDes {
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.tos_ebits);
        bytes.extend_from_slice(&self.tos_metric);
        bytes.extend_from_slice(&self.tos_forwarding_address.to_be_bytes());
        bytes.extend_from_slice(&self.tos_external_route_tag.to_be_bytes());
        bytes
    }
    pub fn length(&self) -> usize {
        12
    }
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        if payload.len() < 12 {
            return None;
        }
        Some(Self {
            tos_ebits: payload[0],
            tos_metric: [payload[1], payload[2], payload[3]],
            tos_forwarding_address: u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]),
            tos_external_route_tag: u32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]),
        })
    }

}

impl ASExternalLSA {
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_be_bytes());
        bytes.extend_from_slice(&self.network_mask.to_be_bytes());
        bytes.push(self.e_bits);
        bytes.extend_from_slice(&self.metric);
        bytes.extend_from_slice(&self.forwarding_address.to_be_bytes());
        bytes.extend_from_slice(&self.external_route_tag.to_be_bytes());
        for tos in &self.tos {
            bytes.extend_from_slice(&tos.to_be_bytes());
        }
        bytes
    }
    pub fn length(&self) -> usize {
        let mut length = Header::length() + 13;
        for tos in &self.tos {
            length += tos.length();
        }
        length
    }
}