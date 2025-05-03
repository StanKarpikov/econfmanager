use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::sync::{Arc, Mutex};

use log::{debug, info};
use socket2::{Domain, Protocol, Socket, Type};

use crate::constants::{MULTICAST_GROUP, MULTICAST_PORT};
use crate::generated::ParameterId;

use crate::interface::SharedRuntimeData;

#[derive (Clone)]
pub(crate) struct EventReceiver {
    runtime_data: Arc<Mutex<SharedRuntimeData>>
}

impl EventReceiver {

    pub(crate) fn new(runtime_data: Arc<Mutex<SharedRuntimeData>>) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = EventReceiver{runtime_data};
        let thread_instance = instance.clone();
        let _ = std::thread::spawn(move || {
            if let Err(e) = thread_instance.multicast_receiver(MULTICAST_GROUP, MULTICAST_PORT) {
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
        
        info!("Waiting for multicast messages...");
        let mut buf = [0u8; 1024];
        loop {
            let (num_bytes, src) = socket.recv_from(&mut buf)?;
            let message = std::str::from_utf8(&buf[..num_bytes])
                .unwrap_or("[non-utf8 data]");
            debug!("Received from {}: {}", src, message);

            self.notify_callback(ParameterId::DEVICE_DEVICE_NAME);
        }
    }

    pub(crate) fn notify_callback(&self, id: ParameterId) {
        let index = id as usize;
        let callback;
        {
            let mut data = self.runtime_data.lock().unwrap();
            // Invalidate the cache so the next time the parameter is read it will be updated from the database
            data.parameters_data[index].value = None;
            callback = data.parameters_data[index].callback;
        }
        if callback.is_some() {
            callback.unwrap()(id);
        }
    }
}