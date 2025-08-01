// src/bin/client.rs

use std::net::UdpSocket;
// Note: We use 'Styx::' to refer to our library crate.
use Styx::packet::{StyxPacket, SYN, ACK};
use Styx::state::ConnectionState;

fn main() -> std::io::Result<()> {
    // Bind to a local address. The OS will pick an available port.
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    // The server's address we want to send data to.
    let server_addr = "127.0.0.1:8081";

    let mut state = ConnectionState::Closed;
    let mut client_isn: u32 = 0;

    // The main state machine loop for the client.
    loop {
        println!("Client state: {:?}", state);
        match state {
            ConnectionState::Closed => {
                // In the Closed state, we begin the handshake by sending a SYN packet.
                client_isn = 100; // Set a random Initial Sequence Number
                let syn_packet = StyxPacket {
                    sequence_number: client_isn,
                    ack_number: 0,
                    flags: SYN,
                    payload: Vec::new(),
                };
                println!("1. Sending SYN...");
                socket.send_to(&syn_packet.to_bytes(), server_addr)?;
                // Transition to the SynSent state to wait for a reply.
                state = ConnectionState::SynSent;
            }
            ConnectionState::SynSent => {
                // In the SynSent state, we are waiting for a SYN-ACK from the server.
                socket.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
                let mut buf = [0; 1024];
                match socket.recv_from(&mut buf) {
                    Ok((bytes_read, _)) => {
                        if let Ok(packet) = StyxPacket::from_bytes(&buf[..bytes_read]) {
                            // Check if the packet is a valid SYN-ACK.
                            if packet.flags == (SYN | ACK) && packet.ack_number == client_isn + 1 {
                                println!("2. Received SYN-ACK: {:?}", packet);
                                // Send the final ACK to complete the handshake.
                                let server_isn = packet.sequence_number;
                                let ack_packet = StyxPacket {
                                    sequence_number: client_isn + 1,
                                    ack_number: server_isn + 1,
                                    flags: ACK,
                                    payload: Vec::new(),
                                };
                                println!("3. Sending ACK...");
                                socket.send_to(&ack_packet.to_bytes(), server_addr)?;
                                // The handshake is complete. Transition to Established.
                                state = ConnectionState::Established;
                            } else {
                                eprintln!("Error: Received invalid SYN-ACK. Closing connection.");
                                state = ConnectionState::Closed;
                                break; // Exit loop
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Timeout waiting for SYN-ACK: {}. Closing connection.", e);
                        state = ConnectionState::Closed;
                        break; // Exit loop
                    }
                }
            }
            ConnectionState::Established => {
                // The connection is established. For now, we just print a message and exit.
                println!("\nHandshake successful! Connection Established.");
                // In a real application, data transfer would happen here.
                state = ConnectionState::Closed; // For this example, we close immediately.
                break; // Exit loop
            }
            // Other states are not handled by the client in this simple example.
            _ => {
                eprintln!("Unhandled state: {:?}. Closing.", state);
                state = ConnectionState::Closed;
                break;
            }
        }
    }
    Ok(())
}
