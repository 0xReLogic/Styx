use crate::packet::{StyxPacket, ACK, FIN, SYN};
use crate::state::ConnectionState;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const DATA_TIMEOUT: Duration = Duration::from_millis(500);

/// A reliable socket built on top of UDP.
pub struct StyxSocket {
    socket: UdpSocket,
    peer_addr: Option<SocketAddr>,
    state: ConnectionState,
    sequence_number: u32,
    ack_number: u32,
}

impl StyxSocket {
    /// Binds the socket to a local address.
    pub fn bind(addr: &str) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_read_timeout(None)?; // Listen indefinitely
        Ok(StyxSocket {
            socket,
            peer_addr: None,
            state: ConnectionState::Listen,
            sequence_number: 0, // Initialized during handshake
            ack_number: 0,
        })
    }

    /// Listens for an incoming connection and performs the 3-way handshake.
    /// Returns a new StyxSocket for the established connection.
    pub fn listen_and_accept(&self) -> std::io::Result<Self> {
        let mut buf = [0; 1024];
        println!("Server is in Listen state, waiting for SYN...");

        // 1. Wait for SYN
        let (amt, src) = self.socket.recv_from(&mut buf)?;
        let received_packet = StyxPacket::from_bytes(&buf[..amt]).unwrap();

        if received_packet.flags == SYN {
            println!("1. Received SYN from {}: {:?}", src, received_packet);
            let client_isn = received_packet.sequence_number;

            // Create a new socket for the connection
            let new_socket = self.socket.try_clone()?;
            new_socket.connect(src)?;
            new_socket.set_read_timeout(Some(HANDSHAKE_TIMEOUT))?;

            let mut connection = StyxSocket {
                socket: new_socket,
                peer_addr: Some(src),
                state: ConnectionState::SynReceived,
                sequence_number: rand::random::<u32>() % 1000, // Server's ISN
                ack_number: client_isn + 1,
            };

            // 2. Send SYN-ACK
            let syn_ack_packet = StyxPacket {
                sequence_number: connection.sequence_number,
                ack_number: connection.ack_number,
                flags: SYN | ACK,
                payload: Vec::new(),
            };
            println!("2. Sending SYN-ACK...");
            connection.socket.send(&syn_ack_packet.to_bytes())?;

            // 3. Wait for final ACK
            let (amt, _) = connection.socket.recv_from(&mut buf)?;
            let final_ack_packet = StyxPacket::from_bytes(&buf[..amt]).unwrap();

            if final_ack_packet.flags == ACK && final_ack_packet.ack_number == connection.sequence_number + 1 {
                println!("3. Received final ACK. Handshake successful!");
                connection.state = ConnectionState::Established;
                connection.sequence_number += 1; // IMPORTANT: Increment sequence number after SYN is ACK'd
                return Ok(connection);
            } else {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid final ACK"));
            }
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Expected SYN packet"))
        }
    }

    /// Connects to a remote address.
    pub fn connect(addr: &str) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("127.0.0.1:0")?;
        socket.connect(addr)?;
        socket.set_read_timeout(Some(HANDSHAKE_TIMEOUT))?;

        let client_isn = rand::random::<u32>() % 1000;
        let mut connection = StyxSocket {
            socket,
            peer_addr: Some(addr.parse().unwrap()),
            state: ConnectionState::SynSent,
            sequence_number: client_isn,
            ack_number: 0,
        };

        // 1. Send SYN
        let syn_packet = StyxPacket {
            sequence_number: client_isn,
            ack_number: 0,
            flags: SYN,
            payload: Vec::new(),
        };
        println!("1. Sending SYN...");
        connection.socket.send(&syn_packet.to_bytes())?;

        // 2. Wait for SYN-ACK
        let mut buf = [0; 1024];
        let (amt, _) = connection.socket.recv_from(&mut buf)?;
        let syn_ack_packet = StyxPacket::from_bytes(&buf[..amt]).unwrap();

        if syn_ack_packet.flags == (SYN | ACK) && syn_ack_packet.ack_number == client_isn + 1 {
            println!("2. Received SYN-ACK: {:?}", syn_ack_packet);
            connection.state = ConnectionState::Established;
            connection.ack_number = syn_ack_packet.sequence_number + 1;
            connection.sequence_number = syn_ack_packet.ack_number;

            // 3. Send final ACK
            let ack_packet = StyxPacket {
                sequence_number: connection.sequence_number,
                ack_number: connection.ack_number,
                flags: ACK,
                payload: Vec::new(),
            };
            println!("3. Sending final ACK...");
            connection.socket.send(&ack_packet.to_bytes())?;

            println!("Handshake successful! Connection Established.");
            return Ok(connection);
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid SYN-ACK"))
        }
    }

    /// Returns the socket address of the remote peer, if connected.
    pub fn peer_addr(&self) -> Option<SocketAddr> {
        self.peer_addr
    }

    pub fn send(&mut self, data: &[u8]) -> std::io::Result<()> {
        let data_packet = StyxPacket {
            sequence_number: self.sequence_number,
            ack_number: self.ack_number,
            flags: 0, // A pure data packet has no flags
            payload: data.to_vec(),
        };
        let packet_bytes = data_packet.to_bytes();

        self.socket.set_read_timeout(Some(DATA_TIMEOUT))?;

        loop {
            self.socket.send(&packet_bytes)?;
            println!("  -> Sent data (seq: {}), waiting for ACK...", self.sequence_number);

            let mut buf = [0; 1024];
            match self.socket.recv_from(&mut buf) {
                Ok((amt, _)) => {
                    if let Ok(ack_packet) = StyxPacket::from_bytes(&buf[..amt]) {
                        if (ack_packet.flags & ACK) != 0 && ack_packet.ack_number == self.sequence_number + 1 {
                            self.socket.set_read_timeout(None)?;
                            self.sequence_number += 1; // Increment for the next packet we send
                            self.ack_number = ack_packet.sequence_number + 1;
                            println!("  <- Received ACK for seq {}.", self.sequence_number - 1);
                            return Ok(());
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    println!("  -> ACK not received, retransmitting...");
                    continue; // Timeout, retransmit
                }
                Err(e) => return Err(e), // Other error
            }
        }
    }

    pub fn recv(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.socket.set_read_timeout(None)?; // Wait indefinitely for data

        loop {
            let (amt, _) = self.socket.recv_from(buf)?;
            if let Ok(packet) = StyxPacket::from_bytes(&buf[..amt]) {
                // If it's a data packet (not FIN, not pure ACK)
                if (packet.flags & (FIN | SYN)) == 0 {
                    println!("  <- Received data (seq: {}), sending ACK.", packet.sequence_number);
                    self.ack_number = packet.sequence_number + 1;

                    let ack_packet = StyxPacket {
                        sequence_number: self.sequence_number,
                        ack_number: self.ack_number,
                        flags: ACK,
                        payload: Vec::new(),
                    };
                    self.socket.send(&ack_packet.to_bytes())?;
                    return Ok(amt);
                } else if (packet.flags & FIN) != 0 {
                    // If we received a FIN while expecting data, pass it up.
                    return Ok(amt);
                }
                // Ignore other packets like stray ACKs from previous transmissions
            }
        }
    }

        pub fn handle_passive_close(&mut self, client_fin_packet: StyxPacket) -> std::io::Result<()> {
        // 1. Send ACK for client's FIN
        self.state = ConnectionState::CloseWait;
        let ack_packet = StyxPacket {
            sequence_number: client_fin_packet.ack_number, // Our seq is their ack
            ack_number: client_fin_packet.sequence_number + 1,
            flags: ACK,
            payload: Vec::new(),
        };
        println!("  - Sending ACK for FIN...");
        self.socket.send(&ack_packet.to_bytes())?;

        // 2. Send our own FIN
        self.state = ConnectionState::LastAck;
        let fin_packet = StyxPacket {
            sequence_number: ack_packet.sequence_number, // Sequence number continues
            ack_number: 0, // Not acknowledging anything
            flags: FIN,
            payload: Vec::new(),
        };
        println!("  - Sending our own FIN...");
        self.socket.send(&fin_packet.to_bytes())?;

        // 3. Wait for final ACK
        let mut buf = [0; 1024];
        let (amt, _) = self.socket.recv_from(&mut buf)?;
        let final_ack = StyxPacket::from_bytes(&buf[..amt]).unwrap();

        if (final_ack.flags & ACK) != 0 && final_ack.ack_number == fin_packet.sequence_number + 1 {
            println!("  - Received final ACK. Connection is closed.");
            self.state = ConnectionState::Closed;
            Ok(())
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Did not receive final ACK"))
        }
    }

    pub fn close(&mut self) -> std::io::Result<()> {
        // 1. Send FIN (Client enters FinWait1)
        self.state = ConnectionState::FinWait1;
        let fin_packet = StyxPacket {
            sequence_number: self.sequence_number,
            ack_number: 0, // Not acknowledging anything
            flags: FIN,
            payload: Vec::new(),
        };
        println!("4. Sending FIN...");
        self.socket.send(&fin_packet.to_bytes())?;

        // 2. Wait for ACK from server
        let mut buf = [0; 1024];
        let (amt, _) = self.socket.recv_from(&mut buf)?;
        let ack_packet = StyxPacket::from_bytes(&buf[..amt]).unwrap();

        if ack_packet.flags == ACK && ack_packet.ack_number == self.sequence_number + 1 {
            self.state = ConnectionState::FinWait2;
            println!("5. Received ACK for FIN.");
        } else {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Did not receive valid ACK for FIN"));
        }

        // 3. Wait for FIN from server
        let (amt, _) = self.socket.recv_from(&mut buf)?;
        let server_fin_packet = StyxPacket::from_bytes(&buf[..amt]).unwrap();

        if (server_fin_packet.flags & FIN) != 0 {
            println!("6. Received FIN from server.");
            self.ack_number = server_fin_packet.sequence_number + 1;

            // 4. Send final ACK (Client enters TimeWait)
            let final_ack_packet = StyxPacket {
                sequence_number: ack_packet.ack_number, // Our sequence number is their ack number
                ack_number: self.ack_number,
                flags: ACK,
                payload: Vec::new(),
            };
            println!("7. Sending final ACK...");
            self.socket.send(&final_ack_packet.to_bytes())?;
            self.state = ConnectionState::TimeWait;

            println!("Entering TimeWait state...");
            // In a real implementation, we'd wait here without blocking the whole program.
            // For this simulation, a simple sleep is fine.
            std::thread::sleep(Duration::from_secs(2));
            self.state = ConnectionState::Closed;
            println!("Connection closed successfully.");
            Ok(())
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Expected FIN from server"))
        }
    }
}
