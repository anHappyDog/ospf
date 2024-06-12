use std::net;


enum RouteDestinationType {
    Network,
    Host,
}


pub struct RouteTableEntry {
    destinationType : RouteDestinationType,
}
