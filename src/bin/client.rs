// src/bin/client.rs

use std::net::UdpSocket;
// Note: We use 'Styx::' to refer to our library crate.
use std::collections::HashSet;
use std::time::{Duration, Instant};
use Styx::packet::{StyxPacket, SYN, ACK, FIN};
use Styx::state::ConnectionState;

fn main() -> std::io::Result<()> {
    // Bind to a local address. The OS will pick an available port.
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    // The server's address we want to send data to.
    let server_addr = "127.0.0.1:8081";

    let mut state = ConnectionState::Closed;
    let mut client_isn: u32 = 0;
    let mut next_seq_num: u32 = 0;

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
                                break; // Exit loop
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Timeout waiting for SYN-ACK: {}. Closing connection.", e);
                        break; // Exit loop
                    }
                }
            }
            ConnectionState::Established => {
                println!("\nHandshake successful! Connection Established.");
                println!("Starting data transfer with Go-Back-N...");

                const WINDOW_SIZE: u32 = 4;
                const TOTAL_PACKETS_TO_SEND: u32 = 10;
                const RETRANSMISSION_TIMEOUT: Duration = Duration::from_secs(1);

                let final_ack_num = client_isn + 1 + TOTAL_PACKETS_TO_SEND;

                let mut base = client_isn + 1;
                next_seq_num = base;
                
                let mut sent_packets: Vec<StyxPacket> = Vec::new();
                let mut acks_received = HashSet::new();
                let mut timer: Option<Instant> = None;

                socket.set_read_timeout(Some(Duration::from_millis(10)))?;

                while base < final_ack_num {
                    // --- Retransmission Timer Check ---
                    if let Some(start_time) = timer {
                        if start_time.elapsed() > RETRANSMISSION_TIMEOUT {
                            println!("\n--- Timeout! Retransmitting from base: {} ---", base);
                            // Go-Back-N: Resend all packets from base onwards.
                            for packet in &sent_packets {
                                if packet.sequence_number >= base {
                                    println!("Retransmitting packet with seq_num: {}", packet.sequence_number);
                                    socket.send_to(&packet.to_bytes(), server_addr)?;
                                }
                            }
                            // Restart the timer
                            timer = Some(Instant::now());
                        }
                    }

                    // --- Sending Logic ---
                    while next_seq_num < base + WINDOW_SIZE && next_seq_num < final_ack_num {
                        let payload = format!("Data packet {}", next_seq_num);
                        let data_packet = StyxPacket {
                            sequence_number: next_seq_num,
                            ack_number: 0,
                            flags: 0,
                            payload: payload.into_bytes(),
                        };
                        println!("Sending packet with seq_num: {}", next_seq_num);
                        socket.send_to(&data_packet.to_bytes(), server_addr)?;
                        sent_packets.push(data_packet); // Buffer the sent packet
                        
                        if timer.is_none() {
                            timer = Some(Instant::now()); // Start timer if it's not already running
                        }
                        next_seq_num += 1;
                    }

                    // --- Receiving Logic ---
                    let mut buf = [0; 1024];
                    match socket.recv_from(&mut buf) {
                        Ok((bytes_read, _)) => {
                            if let Ok(ack) = StyxPacket::from_bytes(&buf[..bytes_read]) {
                                if ack.flags == ACK && ack.ack_number >= base {
                                    println!("Received ACK for seq_num: {}", ack.ack_number);
                                    acks_received.insert(ack.ack_number);

                                    // Slide the window forward
                                    let old_base = base;
                                    while acks_received.contains(&base) {
                                        base += 1;
                                    }
                                    if base > old_base {
                                        println!("Window slided. New base: {}. Resetting timer.", base);
                                        timer = Some(Instant::now()); // Reset timer on successful slide
                                    }
                                }
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {}
                        Err(e) => {
                            eprintln!("Socket error receiving ACK: {}. Aborting.", e);
                            break;
                        }
                    }

                    // If all packets are acknowledged, stop the timer.
                    if base == final_ack_num {
                        timer = None;
                    }
                }

                println!("\nData transfer complete. All packets acknowledged.");
                // Data transfer is done, now let's close the connection gracefully.
                state = ConnectionState::FinWait1;
                // Don't break here, let the main loop handle the new state.
            }

            ConnectionState::FinWait1 => {
                // Active close: send a FIN packet.
                let fin_packet = StyxPacket {
                    sequence_number: next_seq_num, // Continue sequence
                    ack_number: 0,
                    flags: FIN,
                    payload: Vec::new(),
                };
                println!("4. Sending FIN...");
                socket.send_to(&fin_packet.to_bytes(), server_addr)?;

                // Wait for an ACK for our FIN.
                socket.set_read_timeout(Some(Duration::from_secs(5)))?;
                let mut buf = [0; 1024];
                match socket.recv_from(&mut buf) {
                    Ok((bytes_read, _)) => {
                        if let Ok(packet) = StyxPacket::from_bytes(&buf[..bytes_read]) {
                            if packet.flags == ACK && packet.ack_number == next_seq_num + 1 {
                                println!("5. Received ACK for FIN.");
                                state = ConnectionState::FinWait2;
                            } else {
                                eprintln!("Error: Did not receive a valid ACK for FIN. Got: {:?}", packet);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Timeout waiting for FIN-ACK: {}. Closing.", e);
                        break;
                    }
                }
            }

            ConnectionState::FinWait2 => {
                // Now we wait for the server to send its own FIN.
                println!("Waiting for server's FIN...");
                socket.set_read_timeout(Some(Duration::from_secs(10)))?;
                let mut buf = [0; 1024];
                match socket.recv_from(&mut buf) {
                    Ok((bytes_read, _)) => {
                        if let Ok(packet) = StyxPacket::from_bytes(&buf[..bytes_read]) {
                            if packet.flags == FIN {
                                println!("6. Received FIN from server.");
                                // Acknowledge the server's FIN.
                                let final_ack = StyxPacket {
                                    sequence_number: packet.ack_number, // Our seq num is their ack num
                                    ack_number: packet.sequence_number + 1, // We ack their seq num
                                    flags: ACK,
                                    payload: Vec::new(),
                                };
                                println!("7. Sending final ACK...");
                                socket.send_to(&final_ack.to_bytes(), server_addr)?;
                                state = ConnectionState::TimeWait;
                            } else {
                                eprintln!("Error: Expected FIN from server, got: {:?}", packet);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Timeout waiting for server's FIN: {}. Closing.", e);
                        break;
                    }
                }
            }

            ConnectionState::TimeWait => {
                // Wait for a short period to ensure the final ACK is received by the server.
                println!("Entering TimeWait state for 2 seconds...");
                std::thread::sleep(Duration::from_secs(2));
                println!("Connection closed successfully.");
                break; // All done, exit the loop.
            }
            // Other states are not handled by the client in this simple example.
            _ => {
                eprintln!("Unhandled state: {:?}. Closing.", state);
                break;
            }
        }
    }
    Ok(())
}
