# lau-a2ui-protocol

> The wire format for multi-target UI state — bincode payloads, CRC32 checksums, and typed packets for state synchronization.

## What This Does

This crate defines the low-level wire protocol for sending rendering-agnostic UI state between processes. While `lau-a2ui` defines the *state model*, this crate defines how that state gets packed into binary packets with version headers, type discriminators, length prefixes, and CRC32 integrity checks. It also defines the request/response model (`RenderRequest`, `RenderOptions`, `Viewport`) for asking a renderer to produce output.

## The Key Idea

One packet format, many payloads. An `A2UIPacket` wraps any serializable payload (UI state, render requests, control actions, heartbeats) in a binary frame: `[version:1][type:1][len:4 LE][payload:N][crc32:4 LE]`. The CRC32 uses the IEEE polynomial with a compile-time lookup table. On decode, version is checked, payload length validated, and checksum verified. This gives you a clean binary framing layer for any transport (TCP, WebSocket, Unix socket, etc.).

## Install

```bash
cargo add lau-a2ui-protocol
```

## Quick Start

```rust
use lau_a2ui_protocol::*;

// Build state
let state = sample_ui_state();

// Serialize state to bincode
let payload = state.serialize().unwrap();

// Wrap in a packet (auto-computes CRC32)
let packet = A2UIPacket::new(PacketType::StateUpdate, payload);

// Encode to wire bytes
let wire = packet.encode();

// Decode from wire bytes
let decoded = A2UIPacket::decode(&wire).unwrap();
assert_eq!(packet, decoded);

// Recover state from payload
let recovered = UIState::deserialize(&decoded.payload).unwrap();
assert_eq!(state, recovered);
```

## API Reference

### Core Types

| Type | Description |
|------|-------------|
| `UIState` | The canonical rendering-agnostic UI state. Fields: `scene_id`, `timestamp`, `rooms`, `agents`, `notifications`, `metrics`. |
| `RoomState` | Room state: `id`, `name`, `description`, `gravity` (attention/energy/novelty), `deadband`, `ensign_status`, `tile_count`, `last_activity`, `controls`, `wiki_summary`. |
| `AgentState` | Agent state: `id`, `name`, `agent_type`, `status`, `location`, `energy_remaining`, `level` (1–5), `capabilities`. |
| `Notification` | Notification: `id`, `level`, `message`, `timestamp`, `action`. |
| `SystemMetrics` | System metrics: `uptime_seconds`, `total_ticks`, `conservation_remaining`, `rooms_active`, `messages_queued`, `total_energy_used`, `avg_response_ms`. |
| `Control` | Interactive widget: `id`, `label`, `control_type`, `enabled`. |
| `RenderRequest` | Render request: `state`, `target`, `viewport`, `options`. |
| `A2UIPacket` | Wire-format packet: `version`, `packet_type`, `payload`, `checksum`. |

### Enums

| Type | Description |
|------|-------------|
| `PacketType` | `StateUpdate`, `RenderRequest`, `RenderResponse`, `ControlAction`, `Heartbeat` |
| `RenderTarget` | `Terminal`, `Telegram`, `Dashboard`, `GameEngine { engine }`, `Voice`, `JSON`, `A2A` |
| `ControlType` | `Button`, `Slider { min, max, value }`, `Toggle { on }`, `Text { placeholder }`, `Select { options, selected }` |
| `NotificationLevel` | `Info`, `Warning`, `Alert`, `Critical` |
| `DeadbandLevel` | `Green` (default), `Yellow`, `Red` |
| `A2UIError` | `Serialization`, `InvalidPacket`, `ChecksumMismatch`, `UnsupportedVersion`, `UnknownTarget` |

### Key Methods

#### `UIState`
- `serialize(&self) -> Result<Vec<u8>, A2UIError>` — Bincode encoding.
- `deserialize(data: &[u8]) -> Result<Self, A2UIError>` — Bincode decoding.

#### `A2UIPacket`
- `new(packet_type, payload) -> Self` — Create packet with auto-computed CRC32.
- `encode(&self) -> Vec<u8>` — Wire format: `[ver:1][type:1][len:4][payload:N][crc:4]`.
- `decode(data: &[u8]) -> Result<Self, A2UIError>` — Parse and verify wire bytes.

#### `crc32(data: &[u8]) -> u32` (public)
- IEEE polynomial CRC32 with compile-time lookup table.

### Helper Functions
- `sample_ui_state() -> UIState` — Build a sample state for testing.
- `sample_render_request(state) -> RenderRequest` — Build a sample render request.

## How It Works

**Wire format**: `[version: u8][type: u8][payload_len: u32 LE][payload: &[u8]][checksum: u32 LE]`

**CRC32**: IEEE polynomial (0xEDB88320) with a `const fn` generated 256-entry lookup table. Computed over the header + payload (everything except the checksum itself).

**Decode validation**: Checks minimum length (10 bytes), protocol version (currently 1), payload length consistency, and checksum match. Returns typed errors for each failure mode.

**Serialization**: Uses bincode for compact binary encoding of all state types. No schema overhead — the types are self-describing via serde.

## The Math

CRC32 with IEEE polynomial provides burst error detection:
- All single-bit errors detected
- All double-bit errors detected (within 2^32-1 window)
- All odd numbers of errors detected
- All bursts ≤ 32 bits detected
- Probability of undetected random error ≈ 2^(-32)

## Testing

**50+ tests** covering:
- UIState bincode round-trips (full, empty, garbage input)
- All enum variant round-trips (DeadbandLevel, ControlType, NotificationLevel, RenderTarget)
- Packet encode/decode for all packet types
- Empty and large payloads
- Error cases: too short, bad version, checksum mismatch, truncated payload, unknown type byte
- Control interaction simulation
- Full pipeline: state → packet → wire → decode → state
- Multi-target rendering simulation
- CRC32 correctness (known value: `crc32("123456789") == 0xCBF43926`)

## License

MIT
