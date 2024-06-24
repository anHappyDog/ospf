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

pub enum OspfPacket {
    Hello(hello::Hello),
    DD(dd::DD),
    LSU(lsu::Lsu),
    LSR(lsr::Lsr),
    LSACK(lsack::Lsack),
}

impl OspfPacket {
    
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
    pub fn try_from_be_bytes(payload: &[u8]) -> Option<Self> {
        if payload.len() < Self::length() {
            return None;
        }
        Some(Self {
            version: payload[0],
            packet_type: payload[1],
            packet_length: u16::from_be_bytes([payload[2], payload[3]]),
            router_id: u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]),
            area_id: u32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]),
            checksum: u16::from_be_bytes([payload[12], payload[13]]),
            auth_type: payload[14],
            authentication: [
                payload[15],
                payload[16],
                payload[17],
                payload[18],
                payload[19],
                payload[20],
                payload[21],
                payload[22],
            ],
        })
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
