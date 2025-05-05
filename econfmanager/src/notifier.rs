use std::net::UdpSocket;
use log::debug;
use prost::Message;
use crate::generated::ParameterId;
use crate::services::ParameterNotification;
use crate::constants::{MULTICAST_GROUP, MULTICAST_PORT};

#[derive(Default)]
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
        
        debug!("Notification for {}", id as usize);
        let notification = ParameterNotification{id:id as i32};

        let mut buf = Vec::new();
        buf.reserve(notification.encoded_len());
        notification.encode(&mut buf)?;

        socket.send_to(&buf, (MULTICAST_GROUP, MULTICAST_PORT))?;
        
        Ok(())
    }
}