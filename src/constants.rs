use std::net::Ipv4Addr;


pub(crate) const MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 123);
pub(crate) const MULTICAST_PORT: u16 = 44321;