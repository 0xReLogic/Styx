// src/bin/server.rs

use std::net::{UdpSocket, SocketAddr};
// Note: We use 'Styx::' to refer to our library crate.
use Styx::packet::{StyxPacket, SYN, ACK};
use Styx::state::ConnectionState;

fn main() -> std::io::Result<()> {
    // Bind the socket to a local address to listen for packets.
    let socket = UdpSocket::bind("127.0.0.1:8081")?;
    println!("Server listening on 127.0.0.1:8081");

    // Create a buffer to store incoming data.
    let mut buf = [0; 1024];

    let mut state = ConnectionState::Listen;
    let mut client_isn: u32 = 0;
    let mut server_isn: u32 = 0;
    let mut peer_addr: Option<SocketAddr> = None;

    println!("Server is in {:?} state", state);

    // The main state machine loop for the server.
    loop {
        match state {
            ConnectionState::Listen => {
                // In the Listen state, we wait for a SYN packet from a client.
                socket.set_read_timeout(None)?; // Wait indefinitely
                let (number_of_bytes, src_addr) = socket.recv_from(&mut buf)?;
                if let Ok(packet) = StyxPacket::from_bytes(&buf[..number_of_bytes]) {
                    if packet.flags == SYN {
                        println!("1. Received SYN from {}: {:?}", src_addr, packet);
                        client_isn = packet.sequence_number;
                        peer_addr = Some(src_addr); // Store the client's address
                        // Transition to SynReceived to handle the next step.
                        state = ConnectionState::SynReceived;
                    }
                }
            }
            ConnectionState::SynReceived => {
                // In SynReceived, we send a SYN-ACK packet back to the client.
                server_isn = 500; // Set a random Initial Sequence Number
                let syn_ack_packet = StyxPacket {
                    sequence_number: server_isn,
                    ack_number: client_isn + 1,
                    flags: SYN | ACK,
                    payload: Vec::new(),
                };
                println!("2. Sending SYN-ACK...");
                if let Some(addr) = peer_addr {
                    socket.send_to(&syn_ack_packet.to_bytes(), addr)?;
                } else {
                    eprintln!("Error: Peer address not set. Returning to Listen state.");
                    state = ConnectionState::Listen;
                    continue;
                }

                // Now, we wait for the final ACK from the client.
                socket.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
                let mut ack_buf = [0; 1024];
                match socket.recv_from(&mut ack_buf) {
                    Ok((bytes_read, _)) => {
                        if let Ok(ack_packet) = StyxPacket::from_bytes(&ack_buf[..bytes_read]) {
                            if ack_packet.flags == ACK && ack_packet.sequence_number == client_isn + 1 && ack_packet.ack_number == server_isn + 1 {
                                println!("3. Received final ACK.");
                                // The handshake is complete. Transition to Established.
                                state = ConnectionState::Established;
                            } else {
                                eprintln!("Error: Invalid final ACK. Returning to Listen state.");
                                state = ConnectionState::Listen;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Timeout waiting for final ACK: {}. Returning to Listen state.", e);
                        state = ConnectionState::Listen;
                    }
                }
            }
            ConnectionState::Established => {
                println!("\nHandshake successful! Connection Established.");
                println!("Waiting for data...");

                // Loop to receive data packets.
                loop {
                    socket.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
                    let mut data_buf = [0; 1024];
                    match socket.recv_from(&mut data_buf) {
                        Ok((bytes_read, src_addr)) => {
                            if let Ok(data_packet) = StyxPacket::from_bytes(&data_buf[..bytes_read]) {
                                // Check if it's a data packet (no flags set)
                                if data_packet.flags == 0 {


                                    println!("Received data: {:?}", data_packet);

                                    // Send an ACK for the data packet
                                    let ack_packet = StyxPacket {
                                        sequence_number: 0, // Not relevant for this ACK
                                        ack_number: data_packet.sequence_number,
                                        flags: ACK,
                                        payload: Vec::new(),
                                    };
                                    socket.send_to(&ack_packet.to_bytes(), src_addr)?;
                                    println!("Sent ACK for seq_num: {}", data_packet.sequence_number);
                                } else {
                                    // Handle other packet types if necessary, e.g., FIN
                                    println!("Received non-data packet, ignoring for now.");
                                }
                            }
                        }
                        Err(e) => {
                            // If we time out, assume the client is done sending data.
                            println!("Timeout waiting for data: {}. Connection closed.", e);
                            break; // Exit the data receiving loop
                        }
                    }
                }

                println!("--------------------------------------------------");
                // Go back to listening for a new connection.
                state = ConnectionState::Listen;
                println!("Server is in {:?} state", state);
            }
            // Other states are not handled by the server in this simple example.
            _ => {
                eprintln!("Unhandled state: {:?}. Returning to Listen state.", state);
                state = ConnectionState::Listen;
            }
        }
    }
}
