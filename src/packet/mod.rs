pub mod dd;
pub mod hello;
pub mod lsack;
pub mod lsr;
pub mod lsu;

#[derive(Clone, Copy)]
pub struct OspfHeader {
    pub version: u8,
    pub packet_type: u8,
    pub packet_length: u16,
    pub router_id: u32,
    pub area_id: u32,
    pub checksum: u16,
    pub auth_type: u8,
    pub authentication: [u8; 8],
}

impl OspfHeader {
    pub fn empty() -> OspfHeader {
        OspfHeader {
            version: 0,
            packet_type: 0,
            packet_length: 0,
            router_id: 0,
            area_id: 0,
            checksum: 0,
            auth_type: 0,
            authentication: [0; 8],
        }
    }
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.version);
        bytes.push(self.packet_type);
        bytes.extend_from_slice(&self.packet_length.to_be_bytes());
        bytes.extend_from_slice(&self.router_id.to_be_bytes());
        bytes.extend_from_slice(&self.area_id.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.push(self.auth_type);
        bytes.extend_from_slice(&self.authentication);
        bytes
    }
    pub fn length() -> usize {
        24
    }
}
