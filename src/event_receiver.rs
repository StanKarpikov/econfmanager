use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};

use prost::Message;
use socket2::{Domain, Protocol, Socket, Type};

use crate::constants::{MULTICAST_GROUP, MULTICAST_PORT};
use crate::interface::generated::{ParameterId, PARAMETERS_NUM};

use crate::services::ParameterNotification;


pub(crate) struct EventReceiver {
    callbacks: [Option<ParameterUpdateCallback>; PARAMETERS_NUM],
}

type ParameterUpdateCallback = fn(id: ParameterId);

impl EventReceiver {

    pub(crate) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let instance = EventReceiver{callbacks: [None; PARAMETERS_NUM]};
        let _ = std::thread::spawn(move || {
            if let Err(e) = instance.multicast_receiver(MULTICAST_GROUP, MULTICAST_PORT) {
                println!("Receiver error: {}", e);
            }
        });
        Ok(instance)
    }

    pub(crate) fn multicast_receiver(&self, multicast_group: Ipv4Addr, port: u16) -> Result<(), Box<dyn std::error::Error>> {
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

            self.notify_callback(ParameterId::DEVICE_DEVICE_NAME);
        }
    }

    pub(crate) fn add_callback(&mut self, id: ParameterId, callback: ParameterUpdateCallback) -> Result<(), Box<dyn std::error::Error>> {
        let index = id as usize;
        if index < PARAMETERS_NUM {
            self.callbacks[index] = Some(callback);
            Ok(())
        } else {
            Err("Incorrect parameter ID".into())
        }
    }

    pub(crate) fn delete_callback(&mut self, id: ParameterId) -> Result<(), Box<dyn std::error::Error>> {
        let index = id as usize;
        if index < PARAMETERS_NUM {
            self.callbacks[index] = None;
            Ok(())
        } else {
            Err("Incorrect parameter ID".into())
        }
    }

    pub(crate) fn notify_callback(&self, id: ParameterId) {
        let index = id as usize;
        if !self.callbacks[index].is_none() {
            self.callbacks[index].unwrap()(id);
        }
    }
}