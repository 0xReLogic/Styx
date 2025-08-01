// src/packet.rs

// Flags for the StyxPacket header. They can be combined using bitwise OR.
pub const SYN: u8 = 1 << 0; // Synchronize sequence numbers
pub const ACK: u8 = 1 << 1; // Acknowledge
pub const FIN: u8 = 1 << 2; // No more data from sender

/// Represents a single data packet in the Styx protocol.
#[derive(Debug, PartialEq)]
pub struct StyxPacket {
    /// Sequence number of the packet.
    pub sequence_number: u32,
    /// Sequence number of the packet being acknowledged.
    pub ack_number: u32,
    /// Combination of flags (SYN, ACK, FIN).
    pub flags: u8,
    /// The data payload of the packet.
    pub payload: Vec<u8>,
}

const HEADER_SIZE: usize = 9; // 4 (seq) + 4 (ack) + 1 (flags)

impl StyxPacket {
    /// Serializes the StyxPacket into a byte vector.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_SIZE + self.payload.len());
        bytes.extend_from_slice(&self.sequence_number.to_be_bytes());
        bytes.extend_from_slice(&self.ack_number.to_be_bytes());
        bytes.push(self.flags);
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// Deserializes a byte slice into a StyxPacket.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < HEADER_SIZE {
            return Err("Packet too small for header");
        }

        let sequence_number = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        let ack_number = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
        let flags = bytes[8];
        let payload = bytes[HEADER_SIZE..].to_vec();

        Ok(StyxPacket {
            sequence_number,
            ack_number,
            flags,
            payload,
        })
    }
}
