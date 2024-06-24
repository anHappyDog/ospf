pub struct DD {
    pub header: super::OspfHeader,
    pub interface_mtu: u16,
    pub options: u8,
    pub flags: u8,
    pub dd_sequence_number: u32,
    pub lsa_headers: Vec<crate::lsa::Header>,
}
