use crate::area::lsdb;

#[derive(Clone)]
pub struct SummaryLSA {
    pub header: super::Header,
    pub zero_padding: u8,
    pub metric: [u8; 3],
    pub tos: Vec<u32>,
}

pub const SUMMARY_LSA_TYPE_3: u8 = 3;
pub const SUMMARY_LSA_TYPE_4: u8 = 4;

impl SummaryLSA {
    pub fn build_identifier(&self) -> lsdb::LsaIdentifer {
        lsdb::LsaIdentifer {
            lsa_type: self.header.lsa_type as u32,
            link_state_id: self.header.link_state_id,
            advertising_router: self.header.advertising_router,
        }
    }
    pub fn tos_type(tos: u32) -> u8 {
        (tos >> 24) as u8
    }
    pub fn tos_metric(tos: u32) -> u32 {
        (tos & 0x00ffffff) as u32
    }
    pub fn length(&self) -> usize {
        super::Header::length() +  4 + self.tos.len() * 4
    }
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_be_bytes());
        bytes.push(self.zero_padding);
        bytes.extend_from_slice(&self.metric);
        for tos in &self.tos {
            bytes.extend_from_slice(&tos.to_be_bytes());
        }
        bytes
    }
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        if payload.len() < super::Header::length() + 4 {
            return None;
        }
        let header = super::Header::try_from_be_bytes(&payload[..super::Header::length()])?;
        let zero_padding = payload[super::Header::length()];
        let metric = [payload[super::Header::length() + 1], payload[super::Header::length() + 2], payload[super::Header::length() + 3]];
        let mut tos = Vec::new();
        let mut offset = super::Header::length() + 4;
        while offset + 4 <= payload.len() {
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
            header,
            zero_padding,
            metric,
            tos,
        })
    }

}
