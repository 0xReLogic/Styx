// src/bin/client.rs

use std::net::UdpSocket;
// Note: We use 'Styx::' to refer to our library crate.
use Styx::packet::{StyxPacket, SYN, ACK};

fn main() -> std::io::Result<()> {
    // Bind to a local address. The OS will pick an available port.
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    // The server's address we want to send data to.
    let server_addr = "127.0.0.1:8081";

    for i in 0..5 {
        // Create a sample packet to send.
        let packet_to_send = StyxPacket {
            sequence_number: i,
            ack_number: 0,
            flags: SYN,
            payload: format!("Packet number {}", i).into_bytes(),
        };
        println!("Sending packet: {:?}", packet_to_send);

        // Serialize the packet into bytes.
        let bytes_to_send = packet_to_send.to_bytes();

        // Send the data.
        socket.send_to(&bytes_to_send, server_addr)?;

        // Set a timeout for receiving the ACK.
        socket.set_read_timeout(Some(std::time::Duration::from_secs(1)))?;

        // Try to receive the ACK.
        let mut ack_buf = [0; 1024];
        match socket.recv_from(&mut ack_buf) {
            Ok((ack_bytes, _)) => {
                match StyxPacket::from_bytes(&ack_buf[..ack_bytes]) {
                    Ok(ack_packet) => {
                        if ack_packet.flags & ACK != 0 && ack_packet.ack_number == packet_to_send.sequence_number {
                            println!("  -> Received ACK for seq_num: {}", ack_packet.ack_number);
                        } else {
                            println!("  -> Received invalid ACK: {:?}", ack_packet);
                        }
                    }
                    Err(e) => eprintln!("  -> Error deserializing ACK: {}", e),
                }
            }
            Err(e) => {
                eprintln!("  -> Did not receive ACK for seq_num {}: {}", i, e);
            }
        }
    }

    println!("\nFinished sending 5 packets.");

    Ok(())
}
