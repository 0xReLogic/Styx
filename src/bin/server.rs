// src/bin/server.rs

use Styx::packet::{StyxPacket, FIN};
use Styx::styx_socket::StyxSocket;
use std::fs::File;
use std::io::Write;

const SERVER_ADDR: &str = "127.0.0.1:8081";

fn main() -> std::io::Result<()> {
    println!("Server listening on {}", SERVER_ADDR);
    let listener = StyxSocket::bind(SERVER_ADDR)?;

    loop {
        match listener.listen_and_accept() {
            Ok(mut connection) => {
                println!("\nAccepted a new connection from: {:?}", connection.peer_addr());
                
                let mut buffer = [0; 1024];
                let mut file: Option<File> = None;

                // Loop to handle this specific connection
                'connection_loop: loop {
                    match connection.recv(&mut buffer) {
                        Ok(bytes_received) => {
                            let packet = StyxPacket::from_bytes(&buffer[..bytes_received]).unwrap();

                            if (packet.flags & FIN) != 0 {
                                println!("Received FIN, starting passive close.");
                                if let Some(mut f) = file.take() {
                                    f.flush()?;
                                }
                                connection.handle_passive_close(packet)?;
                                break 'connection_loop;
                            }

                            // First data packet is the filename
                            if file.is_none() {
                                let filename = String::from_utf8_lossy(&packet.payload).to_string();
                                println!("Receiving file, will be saved as: '{}'", filename);
                                file = Some(File::create(&filename)?);
                                continue;
                            }
                            
                            // Subsequent packets are file data
                            if !packet.payload.is_empty() {
                                if let Some(ref mut f) = file {
                                    f.write_all(&packet.payload)?;
                                }
                            } else {
                                // Empty packet is EOF
                                println!("EOF received. File transfer complete.");
                                if let Some(mut f) = file.take() {
                                    f.flush()?;
                                }
                                // Now we just wait for the client to send a FIN
                            }
                        }
                        Err(e) => {
                            eprintln!("Error receiving data: {}. Closing connection.", e);
                            break 'connection_loop;
                        }
                    }
                }
                println!("Connection handled and closed. Ready for next connection.");
                println!("--------------------------------------------------");
            }
            Err(e) => {
                eprintln!("Failed to accept connection: {}. Waiting for new connection.", e);
            }
        }
    }
}
