use std::{error::Error, net::{Ipv4Addr, SocketAddrV4, UdpSocket}};
use socket2::{Socket, Domain, Type, Protocol};
use prost::Message;
use crate::interface::generated::ParameterId;
use crate::services::ParameterNotification;
use crate::constants::{MULTICAST_GROUP, MULTICAST_PORT};

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
        
        let notification = ParameterNotification{id:id as i32};

        let mut buf = Vec::new();
        buf.reserve(notification.encoded_len());
        notification.encode(&mut buf)?;

        socket.send_to(&buf, (MULTICAST_GROUP, MULTICAST_PORT))?;
        
        Ok(())
    }
}