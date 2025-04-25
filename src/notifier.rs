const MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 123);
const MULTICAST_PORT: u16 = 44321;

fn multicast_sender() -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    
    // Set Time-to-Live (TTL) for multicast
    socket.set_ttl(1)?;  // Limit to local network
    
    let message = "Hello, multicast!";
    println!("Sending: {}", message);
    socket.send_to(message.as_bytes(), (MULTICAST_GROUP, MULTICAST_PORT))?;
    
    Ok(())
}
