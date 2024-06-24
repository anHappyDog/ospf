pub struct Lsr {
    pub header: super::OspfHeader,
    pub lsa_headers: Vec<crate::lsa::Header>,
}