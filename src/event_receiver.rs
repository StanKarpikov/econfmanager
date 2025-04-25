use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};

use prost::Message;
use socket2::{Domain, Protocol, Socket, Type};

use crate::interface::generated::ParameterId;

mod services {
    include!(concat!(env!("OUT_DIR"), "/", env!("SERVICE_PROTO_FILE_RS")));
}
mod parameter_ids {
    include!(concat!(env!("OUT_DIR"), "/", env!("PARAMETER_IDS_PROTO_FILE_RS")));
}
use services::ParameterNotification;

const MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 123);
const MULTICAST_PORT: u16 = 44321;

// #[derive(Clone, PartialEq, Message)]
// pub struct ParameterNotification {
//     #[prost(message, tag = "1")]
//     pub id: Option<ParameterIdApi>,
// }

pub(crate) struct EventReceiver {

}

impl EventReceiver {

    pub(crate) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let receiver_handle = std::thread::spawn(move || {
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
        
        // Set multicast loopback to receive our own messages (optional)
        socket.set_multicast_loop_v4(true)?;
        
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