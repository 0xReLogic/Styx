// src/bin/client.rs

use std::net::UdpSocket;
// Note: We use 'Styx::' to refer to our library crate.
use Styx::styx_socket::StyxSocket;
use std::fs::File;
use std::io::Read;

const SERVER_ADDR: &str = "127.0.0.1:8081";
const SOURCE_FILE: &str = "sample.txt";
const DESTINATION_FILE: &str = "received_sample.txt";
const CHUNK_SIZE: usize = 512; // 512 bytes per chunk

fn main() -> std::io::Result<()> {
    println!("Attempting to connect to {}", SERVER_ADDR);

    match StyxSocket::connect(SERVER_ADDR) {
        Ok(mut connection) => {
            println!("Successfully connected to the server.");

            // 1. Send destination filename
            println!("Sending destination filename: {}", DESTINATION_FILE);
            connection.send(DESTINATION_FILE.as_bytes())?;

            // 2. Open and send the file in chunks
            let mut file = File::open(SOURCE_FILE)?;
            let mut buffer = [0; CHUNK_SIZE];
            println!("Starting file transfer of '{}'...", SOURCE_FILE);

            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break; // End of file
                }
                connection.send(&buffer[..bytes_read])?;
            }

            // 3. Send EOF signal (empty packet)
            println!("\nFile transfer complete. Sending EOF signal.");
            connection.send(&[])?;

            // 4. Close connection
            println!("Work done. Closing connection.");
            connection.close()?;
        }
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
        }
    }
    Ok(())
}
