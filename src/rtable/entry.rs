enum RouteDestinationType {
    Network,
    Host,
}

pub struct RouteTableEntry {
    destination_type: RouteDestinationType,
}
