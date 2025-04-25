use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};

use prost::Message;
use socket2::{Domain, Protocol, Socket, Type};

use crate::constants::{MULTICAST_GROUP, MULTICAST_PORT};
use crate::interface::generated::ParameterId;

use crate::services::ParameterNotification;


pub(crate) struct EventReceiver {

}

impl EventReceiver {

    pub(crate) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let _ = std::thread::spawn(move || {
            if let Err(e) = Self::multicast_receiver(MULTICAST_GROUP, MULTICAST_PORT) {
                println!("Receiver error: {}", e);
            }
        });
        Ok(EventReceiver{})
    }

    pub(crate) fn multicast_receiver(multicast_group: Ipv4Addr, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let local_addr = Ipv4Addr::new(0, 0, 0, 0);
        
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        socket.bind(&SocketAddrV4::new(local_addr, port).into())?;
        
        // Join the multicast group
        socket.join_multicast_v4(&multicast_group, &local_addr)?;
        
        // Set multicast loopback to not receive our own messages
        socket.set_multicast_loop_v4(false)?;
        
        let socket: UdpSocket = socket.into();
        
        println!("Waiting for multicast messages...");
        let mut buf = [0u8; 1024];
        loop {
            let (num_bytes, src) = socket.recv_from(&mut buf)?;
            let message = std::str::from_utf8(&buf[..num_bytes])
                .unwrap_or("[non-utf8 data]");
            println!("Received from {}: {}", src, message);
        }
    }
}