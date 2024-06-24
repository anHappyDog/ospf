


// pub struct Network {
//     pub id: u32,
//     pub mask: u32,
//     pub area: u32,
//     pub status: Status,
// }


pub struct  Network {
    pub header : super::Header,
    pub network_mask : u32,
    pub attached_routers : Vec<u32>,
}