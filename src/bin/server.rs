// src/bin/server.rs

use std::net::UdpSocket;
// Note: We use 'Styx::' to refer to our library crate.
use Styx::packet::{StyxPacket, ACK};

fn main() -> std::io::Result<()> {
    // Bind the socket to a local address to listen for packets.
    let socket = UdpSocket::bind("127.0.0.1:8081")?;
    println!("Server listening on 127.0.0.1:8081");

    // Create a buffer to store incoming data.
    let mut buf = [0; 1024];

    loop {
        // Block and wait to receive a single packet.
        let (number_of_bytes, src_addr) = socket.recv_from(&mut buf)?;
        println!("\nReceived {} bytes from {}", number_of_bytes, src_addr);

        // Deserialize the received bytes back into a StyxPacket.
        match StyxPacket::from_bytes(&buf[..number_of_bytes]) {
            Ok(packet) => {
                println!("Successfully deserialized packet: {:?}", packet);

                // Create an ACK packet in response.
                let ack_packet = StyxPacket {
                    sequence_number: 0, // Not relevant for a simple ACK
                    ack_number: packet.sequence_number, // Acknowledge the received sequence
                    flags: ACK,
                    payload: Vec::new(), // No payload needed for an ACK
                };

                println!("Sending ACK for seq_num: {}", packet.sequence_number);
                let ack_bytes = ack_packet.to_bytes();
                socket.send_to(&ack_bytes, src_addr)?;
            }
            Err(e) => eprintln!("Error deserializing packet: {}", e),
        }
    }
}
