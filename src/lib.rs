//! # LAU A2UI Protocol
//!
//! The Agent-to-UI wire format — render the same state as a Unity scene,
//! a Telegram message, a terminal dashboard, or a JSON API response.

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Errors that can occur during A2UI protocol operations.
#[derive(Debug, Clone, PartialEq)]
pub enum A2UIError {
    Serialization(String),
    InvalidPacket(String),
    ChecksumMismatch,
    UnsupportedVersion(u8),
    UnknownTarget(String),
}

impl fmt::Display for A2UIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            A2UIError::Serialization(msg) => write!(f, "serialization error: {msg}"),
            A2UIError::InvalidPacket(msg) => write!(f, "invalid packet: {msg}"),
            A2UIError::ChecksumMismatch => write!(f, "checksum mismatch"),
            A2UIError::UnsupportedVersion(v) => write!(f, "unsupported version: {v}"),
            A2UIError::UnknownTarget(t) => write!(f, "unknown render target: {t}"),
        }
    }
}

impl std::error::Error for A2UIError {}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Deadband level indicating system health.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum DeadbandLevel {
    #[default]
    Green,
    Yellow,
    Red,
}

/// Control widget types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ControlType {
    Button,
    Slider {
        min: f64,
        max: f64,
        value: f64,
    },
    Toggle {
        on: bool,
    },
    Text {
        placeholder: String,
    },
    Select {
        options: Vec<String>,
        selected: Option<String>,
    },
}

/// Notification severity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NotificationLevel {
    Info,
    Warning,
    Alert,
    Critical,
}

/// Where a render request targets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RenderTarget {
    Terminal,
    Telegram,
    Dashboard,
    GameEngine { engine: String },
    Voice,
    JSON,
    A2A,
}

/// Packet type discriminator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PacketType {
    StateUpdate,
    RenderRequest,
    RenderResponse,
    ControlAction,
    Heartbeat,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// A single control widget in a room.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Control {
    pub id: String,
    pub label: String,
    pub control_type: ControlType,
    pub enabled: bool,
}

/// State of a single room.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomState {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Attention, energy, novelty.
    pub gravity: [f64; 3],
    pub deadband: DeadbandLevel,
    pub ensign_status: String,
    pub tile_count: usize,
    pub last_activity: u64,
    pub controls: Vec<Control>,
    pub wiki_summary: Option<String>,
}

/// State of a single agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentState {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub status: String,
    pub location: String,
    pub energy_remaining: f64,
    /// Agent level 1–5.
    pub level: u8,
    pub capabilities: Vec<String>,
}

/// A notification to display.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub level: NotificationLevel,
    pub message: String,
    pub timestamp: u64,
    pub action: Option<String>,
}

/// System-wide metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub uptime_seconds: u64,
    pub total_ticks: u64,
    pub conservation_remaining: f64,
    pub rooms_active: usize,
    pub messages_queued: usize,
    pub total_energy_used: f64,
    pub avg_response_ms: f64,
}

/// The canonical rendering-agnostic UI state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UIState {
    pub scene_id: String,
    pub timestamp: u64,
    pub rooms: Vec<RoomState>,
    pub agents: Vec<AgentState>,
    pub notifications: Vec<Notification>,
    pub metrics: SystemMetrics,
}

impl UIState {
    /// Serialize to bincode bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, A2UIError> {
        bincode::serialize(self).map_err(|e| A2UIError::Serialization(e.to_string()))
    }

    /// Deserialize from bincode bytes.
    pub fn deserialize(data: &[u8]) -> Result<Self, A2UIError> {
        bincode::deserialize(data).map_err(|e| A2UIError::Serialization(e.to_string()))
    }
}

/// Client viewport dimensions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
    pub depth: Option<u32>,
    pub format: String,
}

/// Options controlling how a render is produced.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderOptions {
    pub compact: bool,
    pub color: bool,
    pub animations: bool,
    pub max_width: Option<u32>,
    pub include_metrics: bool,
    pub include_controls: bool,
}

/// A client request to render state for a specific target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderRequest {
    pub state: UIState,
    pub target: RenderTarget,
    pub viewport: Viewport,
    pub options: RenderOptions,
}

// ---------------------------------------------------------------------------
// Wire packet
// ---------------------------------------------------------------------------

/// Protocol version.
pub const PROTOCOL_VERSION: u8 = 1;

/// The wire-format packet for A2UI communication.
///
/// Layout: `[version: u8][type: u8][payload_len: u32 LE][payload: &[u8]][checksum: u32 LE]`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct A2UIPacket {
    pub version: u8,
    pub packet_type: PacketType,
    pub payload: Vec<u8>,
    pub checksum: u32,
}

impl A2UIPacket {
    /// Encode the packet to bytes.
    ///
    /// Wire layout: version(1) + type(1) + len(4 LE) + payload(N) + crc32(4 LE)
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(10 + self.payload.len());
        buf.push(self.version);
        buf.push(packet_type_to_byte(&self.packet_type));
        buf.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf.extend_from_slice(&self.checksum.to_le_bytes());
        buf
    }

    /// Decode a packet from bytes.
    pub fn decode(data: &[u8]) -> Result<Self, A2UIError> {
        if data.len() < 10 {
            return Err(A2UIError::InvalidPacket(format!(
                "too short: {} bytes, need at least 10",
                data.len()
            )));
        }
        let version = data[0];
        if version != PROTOCOL_VERSION {
            return Err(A2UIError::UnsupportedVersion(version));
        }
        let pt_byte = data[1];
        let packet_type = byte_to_packet_type(pt_byte)
            .ok_or_else(|| A2UIError::InvalidPacket(format!("unknown packet type byte: {pt_byte}")))?;
        let payload_len = u32::from_le_bytes([data[2], data[3], data[4], data[5]]) as usize;
        if data.len() < 10 + payload_len {
            return Err(A2UIError::InvalidPacket(format!(
                "payload truncation: have {} bytes, need {}",
                data.len() - 6,
                payload_len + 4
            )));
        }
        let payload = data[6..6 + payload_len].to_vec();
        let checksum_offset = 6 + payload_len;
        let checksum = u32::from_le_bytes([
            data[checksum_offset],
            data[checksum_offset + 1],
            data[checksum_offset + 2],
            data[checksum_offset + 3],
        ]);
        // Verify
        let computed = crc32(&data[..checksum_offset]);
        if computed != checksum {
            return Err(A2UIError::ChecksumMismatch);
        }
        Ok(A2UIPacket {
            version,
            packet_type,
            payload,
            checksum,
        })
    }

    /// Create a new packet, computing the checksum automatically.
    pub fn new(packet_type: PacketType, payload: Vec<u8>) -> Self {
        let mut raw = vec![PROTOCOL_VERSION, packet_type_to_byte(&packet_type)];
        raw.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        raw.extend_from_slice(&payload);
        let checksum = crc32(&raw);
        A2UIPacket {
            version: PROTOCOL_VERSION,
            packet_type,
            payload,
            checksum,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn packet_type_to_byte(pt: &PacketType) -> u8 {
    match pt {
        PacketType::StateUpdate => 0x01,
        PacketType::RenderRequest => 0x02,
        PacketType::RenderResponse => 0x03,
        PacketType::ControlAction => 0x04,
        PacketType::Heartbeat => 0x05,
    }
}

fn byte_to_packet_type(b: u8) -> Option<PacketType> {
    match b {
        0x01 => Some(PacketType::StateUpdate),
        0x02 => Some(PacketType::RenderRequest),
        0x03 => Some(PacketType::RenderResponse),
        0x04 => Some(PacketType::ControlAction),
        0x05 => Some(PacketType::Heartbeat),
        _ => None,
    }
}

/// Simple CRC32 (IEEE polynomial) using a lookup table.
pub fn crc32(data: &[u8]) -> u32 {
    const TABLE: [u32; 256] = generate_crc_table();
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        let idx = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = (crc >> 8) ^ TABLE[idx];
    }
    !crc
}

const fn generate_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0u32;
    while i < 256 {
        let mut crc = i;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i as usize] = crc;
        i += 1;
    }
    table
}

// ---------------------------------------------------------------------------
// Test helpers — sample data builders
// ---------------------------------------------------------------------------

pub fn sample_ui_state() -> UIState {
    UIState {
        scene_id: "scene-001".into(),
        timestamp: 1700000000,
        rooms: vec![RoomState {
            id: "room-1".into(),
            name: "Bridge".into(),
            description: "Main command center".into(),
            gravity: [0.8, 0.5, 0.2],
            deadband: DeadbandLevel::Green,
            ensign_status: "nominal".into(),
            tile_count: 12,
            last_activity: 1699999999,
            controls: vec![
                Control {
                    id: "ctrl-1".into(),
                    label: "Engage".into(),
                    control_type: ControlType::Button,
                    enabled: true,
                },
                Control {
                    id: "ctrl-2".into(),
                    label: "Throttle".into(),
                    control_type: ControlType::Slider {
                        min: 0.0,
                        max: 100.0,
                        value: 42.0,
                    },
                    enabled: true,
                },
                Control {
                    id: "ctrl-3".into(),
                    label: "Shields".into(),
                    control_type: ControlType::Toggle { on: true },
                    enabled: false,
                },
                Control {
                    id: "ctrl-4".into(),
                    label: "Target".into(),
                    control_type: ControlType::Text {
                        placeholder: "Enter coordinates…".into(),
                    },
                    enabled: true,
                },
                Control {
                    id: "ctrl-5".into(),
                    label: "Mode".into(),
                    control_type: ControlType::Select {
                        options: vec!["Explore".into(), "Combat".into(), "Stealth".into()],
                        selected: Some("Explore".into()),
                    },
                    enabled: true,
                },
            ],
            wiki_summary: Some("The bridge is where decisions happen.".into()),
        }],
        agents: vec![
            AgentState {
                id: "agent-1".into(),
                name: "ARA".into(),
                agent_type: "pilot".into(),
                status: "active".into(),
                location: "Bridge".into(),
                energy_remaining: 0.87,
                level: 3,
                capabilities: vec!["navigate".into(), "scan".into()],
            },
            AgentState {
                id: "agent-2".into(),
                name: "KOR".into(),
                agent_type: "engineer".into(),
                status: "idle".into(),
                location: "Engine Room".into(),
                energy_remaining: 0.45,
                level: 5,
                capabilities: vec!["repair".into(), "fabricate".into(), "analyze".into()],
            },
        ],
        notifications: vec![
            Notification {
                id: "notif-1".into(),
                level: NotificationLevel::Info,
                message: "System nominal".into(),
                timestamp: 1699999900,
                action: None,
            },
            Notification {
                id: "notif-2".into(),
                level: NotificationLevel::Warning,
                message: "Energy reserves below 50%".into(),
                timestamp: 1699999950,
                action: Some("Reduce non-essential systems".into()),
            },
            Notification {
                id: "notif-3".into(),
                level: NotificationLevel::Alert,
                message: "Anomaly detected in sector 7".into(),
                timestamp: 1699999980,
                action: Some("Investigate".into()),
            },
            Notification {
                id: "notif-4".into(),
                level: NotificationLevel::Critical,
                message: "Hull breach in cargo bay".into(),
                timestamp: 1699999999,
                action: Some("Seal cargo bay immediately".into()),
            },
        ],
        metrics: SystemMetrics {
            uptime_seconds: 86400,
            total_ticks: 1_000_000,
            conservation_remaining: 0.72,
            rooms_active: 5,
            messages_queued: 3,
            total_energy_used: 1234.5,
            avg_response_ms: 12.3,
        },
    }
}

pub fn sample_render_request(state: UIState) -> RenderRequest {
    RenderRequest {
        state,
        target: RenderTarget::Terminal,
        viewport: Viewport {
            width: 80,
            height: 24,
            depth: None,
            format: "text".into(),
        },
        options: RenderOptions {
            compact: false,
            color: true,
            animations: false,
            max_width: Some(80),
            include_metrics: true,
            include_controls: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- UIState serialize/deserialize ----

    #[test]
    fn uistate_roundtrip() {
        let state = sample_ui_state();
        let bytes = state.serialize().unwrap();
        let back = UIState::deserialize(&bytes).unwrap();
        assert_eq!(state, back);
    }

    #[test]
    fn uistate_empty_collections() {
        let state = UIState {
            scene_id: "empty".into(),
            timestamp: 0,
            rooms: vec![],
            agents: vec![],
            notifications: vec![],
            metrics: SystemMetrics {
                uptime_seconds: 0,
                total_ticks: 0,
                conservation_remaining: 0.0,
                rooms_active: 0,
                messages_queued: 0,
                total_energy_used: 0.0,
                avg_response_ms: 0.0,
            },
        };
        let bytes = state.serialize().unwrap();
        let back = UIState::deserialize(&bytes).unwrap();
        assert_eq!(state, back);
    }

    #[test]
    fn deserialize_garbage_errors() {
        let result = UIState::deserialize(b"not valid bincode");
        assert!(matches!(result, Err(A2UIError::Serialization(_))));
    }

    // ---- DeadbandLevel ----

    #[test]
    fn deadband_default_is_green() {
        assert_eq!(DeadbandLevel::default(), DeadbandLevel::Green);
    }

    #[test]
    fn deadband_levels_roundtrip() {
        for level in [DeadbandLevel::Green, DeadbandLevel::Yellow, DeadbandLevel::Red] {
            let bytes = bincode::serialize(&level).unwrap();
            let back: DeadbandLevel = bincode::deserialize(&bytes).unwrap();
            assert_eq!(level, back);
        }
    }

    // ---- ControlType variants ----

    #[test]
    fn control_type_button_roundtrip() {
        let ct = ControlType::Button;
        let bytes = bincode::serialize(&ct).unwrap();
        let back: ControlType = bincode::deserialize(&bytes).unwrap();
        assert_eq!(ct, back);
    }

    #[test]
    fn control_type_slider_roundtrip() {
        let ct = ControlType::Slider { min: -10.0, max: 10.0, value: 3.14 };
        let bytes = bincode::serialize(&ct).unwrap();
        let back: ControlType = bincode::deserialize(&bytes).unwrap();
        assert_eq!(ct, back);
    }

    #[test]
    fn control_type_toggle_roundtrip() {
        for on in [true, false] {
            let ct = ControlType::Toggle { on };
            let bytes = bincode::serialize(&ct).unwrap();
            let back: ControlType = bincode::deserialize(&bytes).unwrap();
            assert_eq!(ct, back);
        }
    }

    #[test]
    fn control_type_text_roundtrip() {
        let ct = ControlType::Text { placeholder: "Type here".into() };
        let bytes = bincode::serialize(&ct).unwrap();
        let back: ControlType = bincode::deserialize(&bytes).unwrap();
        assert_eq!(ct, back);
    }

    #[test]
    fn control_type_select_roundtrip() {
        let ct = ControlType::Select {
            options: vec!["a".into(), "b".into()],
            selected: Some("a".into()),
        };
        let bytes = bincode::serialize(&ct).unwrap();
        let back: ControlType = bincode::deserialize(&bytes).unwrap();
        assert_eq!(ct, back);
    }

    #[test]
    fn control_type_select_none_selected() {
        let ct = ControlType::Select {
            options: vec!["x".into()],
            selected: None,
        };
        let bytes = bincode::serialize(&ct).unwrap();
        let back: ControlType = bincode::deserialize(&bytes).unwrap();
        assert_eq!(ct, back);
    }

    // ---- NotificationLevel ----

    #[test]
    fn notification_level_roundtrip() {
        for level in [
            NotificationLevel::Info,
            NotificationLevel::Warning,
            NotificationLevel::Alert,
            NotificationLevel::Critical,
        ] {
            let bytes = bincode::serialize(&level).unwrap();
            let back: NotificationLevel = bincode::deserialize(&bytes).unwrap();
            assert_eq!(level, back);
        }
    }

    // ---- RenderTarget ----

    #[test]
    fn render_target_roundtrip() {
        let targets = vec![
            RenderTarget::Terminal,
            RenderTarget::Telegram,
            RenderTarget::Dashboard,
            RenderTarget::GameEngine { engine: "Unity".into() },
            RenderTarget::Voice,
            RenderTarget::JSON,
            RenderTarget::A2A,
        ];
        for target in targets {
            let bytes = bincode::serialize(&target).unwrap();
            let back: RenderTarget = bincode::deserialize(&bytes).unwrap();
            assert_eq!(target, back);
        }
    }

    // ---- RenderRequest roundtrip ----

    #[test]
    fn render_request_roundtrip() {
        let req = sample_render_request(sample_ui_state());
        let bytes = bincode::serialize(&req).unwrap();
        let back: RenderRequest = bincode::deserialize(&bytes).unwrap();
        assert_eq!(req, back);
    }

    #[test]
    fn render_request_all_targets() {
        let state = sample_ui_state();
        let targets = vec![
            RenderTarget::Terminal,
            RenderTarget::Telegram,
            RenderTarget::Dashboard,
            RenderTarget::GameEngine { engine: "Unity".into() },
            RenderTarget::Voice,
            RenderTarget::JSON,
            RenderTarget::A2A,
        ];
        for target in targets {
            let req = RenderRequest {
                state: state.clone(),
                target,
                viewport: Viewport {
                    width: 1024,
                    height: 768,
                    depth: None,
                    format: "text".into(),
                },
                options: RenderOptions {
                    compact: false,
                    color: true,
                    animations: false,
                    max_width: None,
                    include_metrics: true,
                    include_controls: true,
                },
            };
            let bytes = bincode::serialize(&req).unwrap();
            let back: RenderRequest = bincode::deserialize(&bytes).unwrap();
            assert_eq!(req, back);
        }
    }

    // ---- Viewport ----

    #[test]
    fn viewport_with_depth() {
        let vp = Viewport {
            width: 1920,
            height: 1080,
            depth: Some(3),
            format: "scene-graph".into(),
        };
        let bytes = bincode::serialize(&vp).unwrap();
        let back: Viewport = bincode::deserialize(&bytes).unwrap();
        assert_eq!(vp, back);
    }

    #[test]
    fn viewport_without_depth() {
        let vp = Viewport {
            width: 80,
            height: 24,
            depth: None,
            format: "text".into(),
        };
        let bytes = bincode::serialize(&vp).unwrap();
        let back: Viewport = bincode::deserialize(&bytes).unwrap();
        assert_eq!(vp, back);
    }

    // ---- RenderOptions ----

    #[test]
    fn render_options_roundtrip() {
        let opts = RenderOptions {
            compact: true,
            color: false,
            animations: true,
            max_width: Some(120),
            include_metrics: false,
            include_controls: true,
        };
        let bytes = bincode::serialize(&opts).unwrap();
        let back: RenderOptions = bincode::deserialize(&bytes).unwrap();
        assert_eq!(opts, back);
    }

    // ---- A2UIPacket encode/decode ----

    #[test]
    fn packet_encode_decode_roundtrip() {
        let payload = b"hello world".to_vec();
        let pkt = A2UIPacket::new(PacketType::StateUpdate, payload);
        let encoded = pkt.encode();
        let decoded = A2UIPacket::decode(&encoded).unwrap();
        assert_eq!(pkt, decoded);
    }

    #[test]
    fn packet_all_types() {
        let types = vec![
            PacketType::StateUpdate,
            PacketType::RenderRequest,
            PacketType::RenderResponse,
            PacketType::ControlAction,
            PacketType::Heartbeat,
        ];
        for pt in types {
            let pkt = A2UIPacket::new(pt, vec![42; 100]);
            let encoded = pkt.encode();
            let decoded = A2UIPacket::decode(&encoded).unwrap();
            assert_eq!(pkt, decoded);
        }
    }

    #[test]
    fn packet_empty_payload() {
        let pkt = A2UIPacket::new(PacketType::Heartbeat, vec![]);
        let encoded = pkt.encode();
        assert_eq!(encoded.len(), 10); // 1+1+4+0+4
        let decoded = A2UIPacket::decode(&encoded).unwrap();
        assert_eq!(pkt, decoded);
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn packet_large_payload() {
        let payload = (0u32..10_000).flat_map(|v| v.to_le_bytes().to_vec()).collect::<Vec<u8>>();
        let pkt = A2UIPacket::new(PacketType::StateUpdate, payload);
        let encoded = pkt.encode();
        let decoded = A2UIPacket::decode(&encoded).unwrap();
        assert_eq!(pkt, decoded);
    }

    #[test]
    fn packet_too_short() {
        let result = A2UIPacket::decode(&[0; 9]);
        assert!(matches!(result, Err(A2UIError::InvalidPacket(_))));
    }

    #[test]
    fn packet_bad_version() {
        let pkt = A2UIPacket::new(PacketType::Heartbeat, vec![]);
        let mut encoded = pkt.encode();
        encoded[0] = 99;
        let result = A2UIPacket::decode(&encoded);
        assert!(matches!(result, Err(A2UIError::UnsupportedVersion(99))));
    }

    #[test]
    fn packet_checksum_mismatch() {
        let pkt = A2UIPacket::new(PacketType::StateUpdate, b"data".to_vec());
        let mut encoded = pkt.encode();
        // Flip a payload byte
        let payload_start = 6;
        if !encoded[payload_start..payload_start + 4].is_empty() {
            encoded[payload_start] ^= 0xFF;
        }
        let result = A2UIPacket::decode(&encoded);
        assert!(matches!(result, Err(A2UIError::ChecksumMismatch)));
    }

    #[test]
    fn packet_truncated_payload() {
        let pkt = A2UIPacket::new(PacketType::StateUpdate, b"hello".to_vec());
        let mut encoded = pkt.encode();
        // Remove last 2 bytes (part of checksum) to simulate truncation
        encoded.truncate(encoded.len() - 2);
        let result = A2UIPacket::decode(&encoded);
        assert!(matches!(result, Err(A2UIError::InvalidPacket(_))));
    }

    #[test]
    fn packet_unknown_type_byte() {
        // version(1) + type(1) + len(4) + payload(0) + crc(4) = 10 bytes
        let mut raw = vec![PROTOCOL_VERSION, 0xFF, 0, 0, 0, 0];
        let checksum = crc32(&raw);
        raw.extend_from_slice(&checksum.to_le_bytes());
        let result = A2UIPacket::decode(&raw);
        assert!(matches!(result, Err(A2UIError::InvalidPacket(_))));
    }

    // ---- RoomState ----

    #[test]
    fn room_state_roundtrip() {
        let room = sample_ui_state().rooms[0].clone();
        let bytes = bincode::serialize(&room).unwrap();
        let back: RoomState = bincode::deserialize(&bytes).unwrap();
        assert_eq!(room, back);
    }

    #[test]
    fn room_state_no_wiki() {
        let room = RoomState {
            id: "r".into(),
            name: "R".into(),
            description: "D".into(),
            gravity: [0.0; 3],
            deadband: DeadbandLevel::Yellow,
            ensign_status: "ok".into(),
            tile_count: 0,
            last_activity: 0,
            controls: vec![],
            wiki_summary: None,
        };
        let bytes = bincode::serialize(&room).unwrap();
        let back: RoomState = bincode::deserialize(&bytes).unwrap();
        assert_eq!(room, back);
    }

    // ---- AgentState ----

    #[test]
    fn agent_state_roundtrip() {
        let agent = sample_ui_state().agents[0].clone();
        let bytes = bincode::serialize(&agent).unwrap();
        let back: AgentState = bincode::deserialize(&bytes).unwrap();
        assert_eq!(agent, back);
    }

    #[test]
    fn agent_state_all_levels() {
        for level in 1..=5u8 {
            let agent = AgentState {
                id: "a".into(),
                name: "A".into(),
                agent_type: "type".into(),
                status: "active".into(),
                location: "here".into(),
                energy_remaining: 1.0,
                level,
                capabilities: vec![],
            };
            let bytes = bincode::serialize(&agent).unwrap();
            let back: AgentState = bincode::deserialize(&bytes).unwrap();
            assert_eq!(agent, back);
        }
    }

    // ---- Notification ----

    #[test]
    fn notification_roundtrip() {
        let notif = sample_ui_state().notifications[0].clone();
        let bytes = bincode::serialize(&notif).unwrap();
        let back: Notification = bincode::deserialize(&bytes).unwrap();
        assert_eq!(notif, back);
    }

    #[test]
    fn notification_with_action() {
        let notif = Notification {
            id: "n".into(),
            level: NotificationLevel::Critical,
            message: "PANIC".into(),
            timestamp: 12345,
            action: Some("Run".into()),
        };
        let bytes = bincode::serialize(&notif).unwrap();
        let back: Notification = bincode::deserialize(&bytes).unwrap();
        assert_eq!(notif, back);
    }

    #[test]
    fn notification_without_action() {
        let notif = Notification {
            id: "n".into(),
            level: NotificationLevel::Info,
            message: "All good".into(),
            timestamp: 0,
            action: None,
        };
        let bytes = bincode::serialize(&notif).unwrap();
        let back: Notification = bincode::deserialize(&bytes).unwrap();
        assert_eq!(notif, back);
    }

    // ---- SystemMetrics ----

    #[test]
    fn system_metrics_roundtrip() {
        let metrics = sample_ui_state().metrics;
        let bytes = bincode::serialize(&metrics).unwrap();
        let back: SystemMetrics = bincode::deserialize(&bytes).unwrap();
        assert_eq!(metrics, back);
    }

    // ---- Control ----

    #[test]
    fn control_roundtrip() {
        let controls = &sample_ui_state().rooms[0].controls;
        for ctrl in controls {
            let bytes = bincode::serialize(ctrl).unwrap();
            let back: Control = bincode::deserialize(&bytes).unwrap();
            assert_eq!(ctrl, &back);
        }
    }

    // ---- Multi-target rendering simulation ----

    #[test]
    fn render_state_for_all_targets() {
        let state = sample_ui_state();
        let targets = [
            RenderTarget::Terminal,
            RenderTarget::Telegram,
            RenderTarget::Dashboard,
            RenderTarget::GameEngine { engine: "Unity".into() },
            RenderTarget::Voice,
            RenderTarget::JSON,
            RenderTarget::A2A,
        ];
        for target in &targets {
            let req = RenderRequest {
                state: state.clone(),
                target: target.clone(),
                viewport: Viewport {
                    width: 80,
                    height: 24,
                    depth: None,
                    format: "text".into(),
                },
                options: RenderOptions {
                    compact: false,
                    color: true,
                    animations: false,
                    max_width: None,
                    include_metrics: true,
                    include_controls: true,
                },
            };
            // Serialize the whole request
            let bytes = bincode::serialize(&req).unwrap();
            let back: RenderRequest = bincode::deserialize(&bytes).unwrap();
            assert_eq!(req, back);
        }
    }

    // ---- Control interaction simulation ----

    #[test]
    fn control_interaction_button_press() {
        let state = sample_ui_state();
        let ctrl = &state.rooms[0].controls[0]; // Button
        assert!(ctrl.enabled);
        assert_eq!(ctrl.control_type, ControlType::Button);
    }

    #[test]
    fn control_interaction_slider_value() {
        let state = sample_ui_state();
        let ctrl = &state.rooms[0].controls[1]; // Slider
        if let ControlType::Slider { value, min, max } = ctrl.control_type {
            assert!(value >= min && value <= max);
        } else {
            panic!("expected Slider");
        }
    }

    #[test]
    fn control_interaction_toggle() {
        let state = sample_ui_state();
        let ctrl = &state.rooms[0].controls[2]; // Toggle
        assert_eq!(ctrl.control_type, ControlType::Toggle { on: true });
        assert!(!ctrl.enabled); // disabled in sample
    }

    #[test]
    fn control_interaction_select() {
        let state = sample_ui_state();
        let ctrl = &state.rooms[0].controls[4]; // Select
        if let ControlType::Select { options, selected } = &ctrl.control_type {
            assert_eq!(options.len(), 3);
            assert_eq!(selected.as_deref(), Some("Explore"));
        } else {
            panic!("expected Select");
        }
    }

    // ---- Packet with serialized UIState ----

    #[test]
    fn packet_with_uistate_payload() {
        let state = sample_ui_state();
        let payload = state.serialize().unwrap();
        let pkt = A2UIPacket::new(PacketType::StateUpdate, payload);
        let encoded = pkt.encode();
        let decoded = A2UIPacket::decode(&encoded).unwrap();
        assert_eq!(pkt, decoded);
        // Verify the payload deserializes back to the original state
        let state_back = UIState::deserialize(&decoded.payload).unwrap();
        assert_eq!(state, state_back);
    }

    #[test]
    fn packet_with_render_request_payload() {
        let req = sample_render_request(sample_ui_state());
        let payload = bincode::serialize(&req).unwrap();
        let pkt = A2UIPacket::new(PacketType::RenderRequest, payload);
        let encoded = pkt.encode();
        let decoded = A2UIPacket::decode(&encoded).unwrap();
        let req_back: RenderRequest = bincode::deserialize(&decoded.payload).unwrap();
        assert_eq!(req, req_back);
    }

    // ---- CRC32 correctness ----

    #[test]
    fn crc32_known_value() {
        // CRC32 of "123456789" is 0xCBF43926
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn crc32_empty() {
        assert_eq!(crc32(b""), 0x0000_0000);
    }

    // ---- Display impl ----

    #[test]
    fn error_display() {
        assert_eq!(
            A2UIError::ChecksumMismatch.to_string(),
            "checksum mismatch"
        );
        assert_eq!(
            A2UIError::UnsupportedVersion(99).to_string(),
            "unsupported version: 99"
        );
        assert_eq!(
            A2UIError::UnknownTarget("fax".into()).to_string(),
            "unknown render target: fax"
        );
        assert_eq!(
            A2UIError::Serialization("bad".into()).to_string(),
            "serialization error: bad"
        );
        assert_eq!(
            A2UIError::InvalidPacket("short".into()).to_string(),
            "invalid packet: short"
        );
    }

    // ---- Full pipeline: state → packet → wire → decode → state ----

    #[test]
    fn full_pipeline() {
        let state = sample_ui_state();
        let payload = state.serialize().unwrap();
        let pkt = A2UIPacket::new(PacketType::StateUpdate, payload);
        let wire = pkt.encode();

        // Simulate transmission
        let decoded_pkt = A2UIPacket::decode(&wire).unwrap();
        let decoded_state = UIState::deserialize(&decoded_pkt.payload).unwrap();

        assert_eq!(state, decoded_state);
        assert_eq!(decoded_pkt.version, PROTOCOL_VERSION);
        assert_eq!(decoded_pkt.packet_type, PacketType::StateUpdate);
    }

    #[test]
    fn full_pipeline_render_request_telegram() {
        let state = sample_ui_state();
        let req = RenderRequest {
            state,
            target: RenderTarget::Telegram,
            viewport: Viewport {
                width: 320,
                height: 200,
                depth: None,
                format: "markdown".into(),
            },
            options: RenderOptions {
                compact: true,
                color: false,
                animations: false,
                max_width: Some(40),
                include_metrics: false,
                include_controls: true,
            },
        };
        let payload = bincode::serialize(&req).unwrap();
        let pkt = A2UIPacket::new(PacketType::RenderRequest, payload);
        let wire = pkt.encode();
        let decoded = A2UIPacket::decode(&wire).unwrap();
        let req_back: RenderRequest = bincode::deserialize(&decoded.payload).unwrap();
        assert_eq!(req, req_back);
    }

    #[test]
    fn full_pipeline_game_engine() {
        let state = sample_ui_state();
        let req = RenderRequest {
            state,
            target: RenderTarget::GameEngine { engine: "Unity".into() },
            viewport: Viewport {
                width: 1920,
                height: 1080,
                depth: Some(3),
                format: "scene-graph".into(),
            },
            options: RenderOptions {
                compact: false,
                color: true,
                animations: true,
                max_width: None,
                include_metrics: true,
                include_controls: true,
            },
        };
        let payload = bincode::serialize(&req).unwrap();
        let pkt = A2UIPacket::new(PacketType::RenderRequest, payload);
        let wire = pkt.encode();
        let decoded = A2UIPacket::decode(&wire).unwrap();
        let req_back: RenderRequest = bincode::deserialize(&decoded.payload).unwrap();
        assert_eq!(req, req_back);
    }

    // ---- Multiple rooms ----

    #[test]
    fn multiple_rooms() {
        let mut state = sample_ui_state();
        state.rooms.push(RoomState {
            id: "room-2".into(),
            name: "Engine Room".into(),
            description: "Where the magic happens".into(),
            gravity: [0.3, 0.9, 0.1],
            deadband: DeadbandLevel::Red,
            ensign_status: "critical".into(),
            tile_count: 8,
            last_activity: 1699999900,
            controls: vec![],
            wiki_summary: None,
        });
        let bytes = state.serialize().unwrap();
        let back = UIState::deserialize(&bytes).unwrap();
        assert_eq!(state, back);
        assert_eq!(back.rooms.len(), 2);
    }

    // ---- Many notifications ----

    #[test]
    fn many_notifications() {
        let mut state = sample_ui_state();
        for i in 0..50 {
            state.notifications.push(Notification {
                id: format!("notif-{i}"),
                level: NotificationLevel::Info,
                message: format!("Message {i}"),
                timestamp: 1699999000 + i,
                action: if i % 2 == 0 { Some(format!("Act {i}")) } else { None },
            });
        }
        let bytes = state.serialize().unwrap();
        let back = UIState::deserialize(&bytes).unwrap();
        assert_eq!(state, back);
        assert_eq!(back.notifications.len(), 54); // 4 original + 50
    }
}
