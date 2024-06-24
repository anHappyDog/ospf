pub const LSR_TYPE : u8 = 3;
pub struct Lsr {
    pub header: super::OspfHeader,
    pub lsa_headers: Vec<crate::lsa::Header>,
}

impl Lsr {
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        unimplemented!()
    }
}

pub async fn when_received(lsr_packet : Lsr) {
    
}