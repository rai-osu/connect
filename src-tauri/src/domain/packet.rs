//! Bancho packet parsing and manipulation.
//!
//! This module provides types and functions for parsing and manipulating
//! Bancho protocol packets. The Bancho protocol is used for communication
//! between the osu! client and the game server.
//!
//! # Packet Format
//!
//! Each Bancho packet has the following structure:
//!
//! | Field       | Size    | Description                    |
//! |-------------|---------|--------------------------------|
//! | packet_id   | 2 bytes | Little-endian packet type ID   |
//! | compression | 1 byte  | Compression flag (usually 0)   |
//! | length      | 4 bytes | Little-endian payload length   |
//! | payload     | varies  | Packet-specific data           |
//!
//! The total header size is 7 bytes.

/// Known server packet IDs in the Bancho protocol.
///
/// This enum covers the packet types that are relevant for the proxy's
/// functionality. Unknown packet types are represented as `Unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ServerPacketId {
    LoginReply = 5,
    ProtocolVersion = 75,
    UserPrivileges = 71,
    UserPresence = 83,
    UserStats = 11,
    ChannelInfo = 64,
    Notification = 24,
    Unknown = 0,
}

impl From<u16> for ServerPacketId {
    fn from(value: u16) -> Self {
        match value {
            5 => Self::LoginReply,
            75 => Self::ProtocolVersion,
            71 => Self::UserPrivileges,
            83 => Self::UserPresence,
            11 => Self::UserStats,
            64 => Self::ChannelInfo,
            24 => Self::Notification,
            _ => Self::Unknown,
        }
    }
}

/// User privilege flags in the Bancho protocol.
///
/// Privileges are stored as a bitfield where each bit represents a different
/// permission level. The `SUPPORTER` flag (bit 2) is particularly important
/// as it enables osu!direct functionality in the client.
///
/// # Example
///
/// ```
/// use crate::domain::Privileges;
///
/// let normal = Privileges::default();
/// assert!(!normal.has_supporter());
///
/// let supporter = normal.with_supporter();
/// assert!(supporter.has_supporter());
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Privileges(pub u32);

impl Privileges {
    /// Normal user privileges (no special permissions).
    pub const NORMAL: u32 = 1;

    /// Supporter status - enables osu!direct and other perks.
    pub const SUPPORTER: u32 = 4;

    /// Beatmap Appreciation Team member.
    pub const BAT: u32 = 2;

    /// Tournament staff permissions.
    pub const TOURNAMENT: u32 = 32;

    /// Returns a new `Privileges` with the supporter flag set.
    pub fn with_supporter(self) -> Self {
        Self(self.0 | Self::SUPPORTER)
    }

    /// Returns `true` if the supporter flag is set.
    pub fn has_supporter(&self) -> bool {
        self.0 & Self::SUPPORTER != 0
    }

    /// Returns the raw privilege value.
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Default for Privileges {
    fn default() -> Self {
        Self(Self::NORMAL)
    }
}

/// Header of a Bancho protocol packet.
///
/// The header is 7 bytes and contains the packet type, compression flag,
/// and payload length. All multi-byte values are little-endian.
#[derive(Debug, Clone)]
pub struct PacketHeader {
    /// The packet type identifier.
    pub packet_id: u16,

    /// Compression flag (0 = uncompressed, 1 = compressed).
    /// Note: Compression is rarely used in practice.
    pub compression: u8,

    /// Length of the payload in bytes.
    pub length: u32,
}

impl PacketHeader {
    /// Size of the packet header in bytes.
    pub const SIZE: usize = 7;

    /// Parses a packet header from a byte slice.
    ///
    /// # Arguments
    ///
    /// * `data` - Byte slice containing at least 7 bytes
    ///
    /// # Returns
    ///
    /// `Some(PacketHeader)` if parsing succeeds, `None` if the slice is too short.
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }

        let packet_id = u16::from_le_bytes([data[0], data[1]]);
        let compression = data[2];
        let length = u32::from_le_bytes([data[3], data[4], data[5], data[6]]);

        Some(Self {
            packet_id,
            compression,
            length,
        })
    }

    /// Serializes the header to bytes.
    ///
    /// # Returns
    ///
    /// A 7-byte array containing the serialized header.
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let id_bytes = self.packet_id.to_le_bytes();
        let len_bytes = self.length.to_le_bytes();

        [
            id_bytes[0],
            id_bytes[1],
            self.compression,
            len_bytes[0],
            len_bytes[1],
            len_bytes[2],
            len_bytes[3],
        ]
    }
}

/// A complete Bancho protocol packet with header and payload.
///
/// Packets are the fundamental unit of communication in the Bancho protocol.
/// Each packet consists of a 7-byte header followed by a variable-length payload.
///
/// # Parsing
///
/// Packets can be parsed from a byte stream using [`Packet::parse_stream`].
/// This handles the common case of TCP fragmentation where multiple packets
/// may arrive in a single read, or a single packet may be split across reads.
#[derive(Debug, Clone)]
pub struct Packet {
    /// The packet header containing type and length information.
    pub header: PacketHeader,

    /// The packet payload data.
    pub payload: Vec<u8>,
}

impl Packet {
    /// Parses complete packets from a byte stream.
    ///
    /// This function handles TCP fragmentation by extracting all complete
    /// packets from the input and returning any remaining incomplete data.
    /// The remaining data should be prepended to the next read.
    ///
    /// # Arguments
    ///
    /// * `data` - Byte slice containing one or more (possibly partial) packets
    ///
    /// # Returns
    ///
    /// A tuple of:
    /// - `Vec<Packet>` - All complete packets that could be parsed
    /// - `Vec<u8>` - Remaining bytes (incomplete packet data)
    ///
    /// # Example
    ///
    /// ```
    /// use crate::domain::Packet;
    ///
    /// let mut buffer = Vec::new();
    ///
    /// // First read - might be partial
    /// buffer.extend_from_slice(&first_read);
    /// let (packets, remaining) = Packet::parse_stream(&buffer);
    /// buffer = remaining;
    ///
    /// // Process complete packets
    /// for packet in packets {
    ///     handle_packet(packet);
    /// }
    /// ```
    pub fn parse_stream(data: &[u8]) -> (Vec<Self>, Vec<u8>) {
        let mut packets = Vec::new();
        let mut offset = 0;

        while offset + PacketHeader::SIZE <= data.len() {
            let header = match PacketHeader::parse(&data[offset..]) {
                Some(h) => h,
                None => break,
            };

            let total_len = PacketHeader::SIZE + header.length as usize;
            if offset + total_len > data.len() {
                break;
            }

            let payload_start = offset + PacketHeader::SIZE;
            let payload_end = payload_start + header.length as usize;
            let payload = data[payload_start..payload_end].to_vec();

            packets.push(Self { header, payload });
            offset += total_len;
        }

        let remaining = data[offset..].to_vec();
        (packets, remaining)
    }

    /// Serializes the packet to bytes.
    ///
    /// # Returns
    ///
    /// A byte vector containing the complete packet (header + payload).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PacketHeader::SIZE + self.payload.len());
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// Returns the packet type as a `ServerPacketId`.
    ///
    /// Unknown packet types are returned as `ServerPacketId::Unknown`.
    pub fn packet_type(&self) -> ServerPacketId {
        ServerPacketId::from(self.header.packet_id)
    }
}

/// Injects supporter privileges into a `UserPrivileges` packet.
///
/// This function modifies the packet in-place to add the `SUPPORTER` flag
/// to the user's privileges. If the packet is not a `UserPrivileges` packet
/// or the payload is too short, the function does nothing.
///
/// # Arguments
///
/// * `packet` - The packet to modify
///
/// # Safety
///
/// This function assumes the payload follows the standard `UserPrivileges`
/// format (4-byte little-endian u32). If the payload format is different,
/// the modification may produce unexpected results.
pub fn inject_supporter_privileges(packet: &mut Packet) {
    if packet.packet_type() != ServerPacketId::UserPrivileges {
        return;
    }

    if packet.payload.len() >= 4 {
        let current = u32::from_le_bytes([
            packet.payload[0],
            packet.payload[1],
            packet.payload[2],
            packet.payload[3],
        ]);

        let privileges = Privileges(current).with_supporter();
        let new_bytes = privileges.value().to_le_bytes();

        packet.payload[0] = new_bytes[0];
        packet.payload[1] = new_bytes[1];
        packet.payload[2] = new_bytes[2];
        packet.payload[3] = new_bytes[3];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_header() {
        let data = [5, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0];
        let header = PacketHeader::parse(&data).unwrap();

        assert_eq!(header.packet_id, 5);
        assert_eq!(header.compression, 0);
        assert_eq!(header.length, 4);
    }

    #[test]
    fn test_privileges_supporter() {
        let priv_normal = Privileges(Privileges::NORMAL);
        assert!(!priv_normal.has_supporter());

        let priv_supporter = priv_normal.with_supporter();
        assert!(priv_supporter.has_supporter());
        assert_eq!(
            priv_supporter.value(),
            Privileges::NORMAL | Privileges::SUPPORTER
        );
    }

    #[test]
    fn test_inject_supporter() {
        let mut packet = Packet {
            header: PacketHeader {
                packet_id: ServerPacketId::UserPrivileges as u16,
                compression: 0,
                length: 4,
            },
            payload: Privileges::NORMAL.to_le_bytes().to_vec(),
        };

        inject_supporter_privileges(&mut packet);

        let new_priv = u32::from_le_bytes([
            packet.payload[0],
            packet.payload[1],
            packet.payload[2],
            packet.payload[3],
        ]);

        assert!(Privileges(new_priv).has_supporter());
    }
}
