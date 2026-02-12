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

#[derive(Debug, Clone, Copy)]
pub struct Privileges(pub u32);

impl Privileges {
    pub const NORMAL: u32 = 1;
    pub const SUPPORTER: u32 = 4;
    pub const BAT: u32 = 2;
    pub const TOURNAMENT: u32 = 32;

    pub fn with_supporter(self) -> Self {
        Self(self.0 | Self::SUPPORTER)
    }

    pub fn has_supporter(&self) -> bool {
        self.0 & Self::SUPPORTER != 0
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Default for Privileges {
    fn default() -> Self {
        Self(Self::NORMAL)
    }
}

#[derive(Debug, Clone)]
pub struct PacketHeader {
    pub packet_id: u16,
    pub compression: u8,
    pub length: u32,
}

impl PacketHeader {
    pub const SIZE: usize = 7;

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

#[derive(Debug, Clone)]
pub struct Packet {
    pub header: PacketHeader,
    pub payload: Vec<u8>,
}

impl Packet {
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

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PacketHeader::SIZE + self.payload.len());
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    pub fn packet_type(&self) -> ServerPacketId {
        ServerPacketId::from(self.header.packet_id)
    }
}

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
