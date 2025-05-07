use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::sync::{Arc, Mutex};

use log::{debug, error, info, warn};
use prost::Message;
use socket2::{Domain, Protocol, Socket, Type};

use crate::constants::{MULTICAST_GROUP, MULTICAST_PORT};
use crate::generated::ParameterId;

use crate::interface::SharedRuntimeData;
use crate::service_events::ParameterNotification;

#[derive (Clone, Default)]
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
        
        info!("Starting multicast receiver on {}:{}", multicast_group, port);
    
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
            .map_err(|e| {
                error!("Socket creation failed: {}", e);
                e
            })?;
    
        socket.set_reuse_address(true)
            .map_err(|e| warn!("SO_REUSEADDR failed (non-fatal): {}", e)).ok();
    
        #[cfg(target_os = "linux")]
        socket.set_reuse_port(true)
            .map_err(|e| warn!("SO_REUSEPORT failed (non-fatal): {}", e)).ok();
    
        socket.bind(&SocketAddrV4::new(local_addr, port).into())
            .map_err(|e| {
                error!("Failed to bind to port {}: {}", port, e);
                e
            })?;
        info!("Successfully bound to UDP port {}", port);
    
        socket.join_multicast_v4(&multicast_group, &local_addr)
            .map_err(|e| {
                error!("Multicast join failed: {}", e);
                e
            })?;
        socket.set_multicast_loop_v4(false)?;
    
        let socket: UdpSocket = socket.into();
        info!("Listening for multicast messages...");
    
        let mut buf = [0u8; 1024];
        loop {
            match socket.recv_from(&mut buf) {
                Ok((num_bytes, src)) => {
                    match ParameterNotification::decode(&buf[..num_bytes]) {
                        Ok(notification) => {
                            debug!("Received parameter notification from {}: id={}", src, notification.id);
                            match ParameterId::try_from(notification.id as usize) {
                                Ok(id) => self.notify_callback(id),
                                Err(e) => {
                                    error!("Could not decode ID {}: {}", notification.id, e);
                                    continue
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to decode ParameterNotification from {}: {}", src, e);
                            // Optionally continue or return error
                            continue;
                        }
                    }
                }
                Err(e) => {
                    error!("Receive error: {}", e);
                    return Err(Box::new(e));
                }
            }
        }
    }

    pub(crate) fn notify_callback(&self, id: ParameterId) {
        let index = id as usize;
        let callback;
        {
            let mut data = self.runtime_data.lock().unwrap();
            // Invalidate the cache so the next time the parameter is read it will be updated from the database
            data.parameters_data[index].value = None;
            callback = data.parameters_data[index].callback.clone();
        }
        if callback.is_some() {
            debug!("Call callback for {}", id as usize);
            callback.unwrap()(id);
        }
        else {
            debug!("Callback for {} not defined", id as usize);
        }
    }
}