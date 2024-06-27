#[derive(Clone)]
pub struct SummaryLSA {
    pub header: super::Header,
    pub zero_padding: u8,
    pub metric: [u8; 3],
    pub tos: Vec<u32>,
}

impl SummaryLSA {
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
}
