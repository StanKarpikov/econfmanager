use std::{error::Error, net::{Ipv4Addr, SocketAddrV4, UdpSocket}};
use socket2::{Socket, Domain, Type, Protocol};

use crate::interface::generated::ParameterId;

const MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 123);
const MULTICAST_PORT: u16 = 44321;

pub(crate) struct Notifier {

}

impl Notifier {
    pub(crate) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Notifier{})
    }

    pub(crate) fn notify_of_parameter_change(&self, id: ParameterId) -> Result<(), Box<dyn std::error::Error>> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        
        // Set Time-to-Live (TTL) for multicast
        socket.set_ttl(1)?;  // Limit to local network
        
        // let notification = ParameterNotification {
        //     id as i32,
        // };

        // let mut buf = Vec::new();
        // notification.encode(&mut buf)?;

        // let message = id;
        // socket.send_to(&buf, (MULTICAST_GROUP, MULTICAST_PORT))?;
        
        Ok(())
    }
}