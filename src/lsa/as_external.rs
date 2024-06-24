

pub struct AsExternal {
    pub header : super::Header,
    pub netmask : u32,
    pub metric : u32,
    pub forwarding_address : u32,
    pub external_route_tag : u32,
}