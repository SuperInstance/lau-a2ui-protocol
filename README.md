# lau-a2ui-protocol

> The wire format for multi-target UI state — bincode payloads, CRC32 checksums, and typed packets for state synchronization.

## What This Does

This crate defines the **low-level wire protocol** for sending rendering-agnostic UI state between processes. While `lau-a2ui` defines the *state model* and renderers, this crate defines how that state gets packed into **binary packets** with:

- **Version headers** — protocol versioning for forward compatibility
- **Type discriminators** — packet type identification (state update, render request, control action, heartbeat)
- **Length-prefixed payloads** — bincode-serialized data with explicit size
- **CRC32 integrity checks** — IEEE polynomial with compile-time lookup table

It also defines the **request/response model** (`RenderRequest`, `RenderOptions`, `Viewport`) for asking a renderer to produce output, plus typed UI state (`UIState`, `RoomState`, `AgentState`, etc.) with bincode serialization.

## The Key Idea

One packet format, many payloads. An `A2UIPacket` wraps any serializable payload in a binary frame:

```
┌──────────┬──────┬───────────┬───────────────┬──────────┐
│ version  │ type │ len (u32) │   payload     │  CRC32   │
│  (1 byte)│(1 B) │  (4 LE)   │   (N bytes)   │ (4 LE)   │
└──────────┴──────┴───────────┴───────────────┴──────────┘
```

The CRC32 covers everything before the checksum itself. On decode, the version is checked, payload length is validated, and the checksum is recomputed and compared. This gives you a clean binary framing layer for any transport (TCP, WebSocket, Unix socket, etc.).

```
  Application
       │
       ▼
  ┌─────────────┐     ┌──────────────┐
  │  UIState    │────→│ A2UIPacket   │────→ wire bytes
  │  serialize()│     │ .new()       │     .encode()
  └─────────────┘     └──────────────┘
                            │
                     ┌──────┴──────┐
                     │   CRC32     │
                     │ (IEEE poly) │
                     └─────────────┘
```

## Install

```bash
cargo add lau-a2ui-protocol
```

**Dependencies:** `serde`, `bincode`

## Quick Start

```rust
use lau_a2ui_protocol::*;

// 1. Build state
let state = sample_ui_state();

// 2. Serialize state to bincode
let payload = state.serialize().unwrap();

// 3. Wrap in a packet (auto-computes CRC32)
let packet = A2UIPacket::new(PacketType::StateUpdate, payload);

// 4. Encode to wire bytes
let wire = packet.encode();

// 5. Decode from wire bytes
let decoded = A2UIPacket::decode(&wire).unwrap();
assert_eq!(packet, decoded);

// 6. Recover state from payload
let recovered = UIState::deserialize(&decoded.payload).unwrap();
assert_eq!(state, recovered);
```

### Sending a Render Request

```rust
use lau_a2ui_protocol::*;

let state = sample_ui_state();
let request = RenderRequest {
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

let payload = bincode::serialize(&request).unwrap();
let packet = A2UIPacket::new(PacketType::RenderRequest, payload);
let wire = packet.encode();

// On the receiving end:
let decoded = A2UIPacket::decode(&wire).unwrap();
let request: RenderRequest = bincode::deserialize(&decoded.payload).unwrap();
```

## API Reference

### Core Types

| Type | Description |
|------|-------------|
| `UIState` | Canonical rendering-agnostic UI state. Fields: `scene_id`, `timestamp`, `rooms`, `agents`, `notifications`, `metrics`. |
| `RoomState` | Room state: `id`, `name`, `description`, `gravity` (attention/energy/novelty as `[f64; 3]`), `deadband`, `ensign_status`, `tile_count`, `last_activity`, `controls`, `wiki_summary`. |
| `AgentState` | Agent state: `id`, `name`, `agent_type`, `status`, `location`, `energy_remaining`, `level` (1–5), `capabilities`. |
| `Notification` | Notification: `id`, `level`, `message`, `timestamp`, `action` (optional). |
| `SystemMetrics` | System metrics: `uptime_seconds`, `total_ticks`, `conservation_remaining`, `rooms_active`, `messages_queued`, `total_energy_used`, `avg_response_ms`. |
| `Control` | Interactive widget: `id`, `label`, `control_type`, `enabled`. |
| `RenderRequest` | Render request: `state`, `target`, `viewport`, `options`. |
| `Viewport` | Client dimensions: `width`, `height`, `depth` (optional), `format`. |
| `RenderOptions` | Render flags: `compact`, `color`, `animations`, `max_width`, `include_metrics`, `include_controls`. |
| `A2UIPacket` | Wire-format packet: `version`, `packet_type`, `payload`, `checksum`. |

### Enums

| Type | Variants |
|------|----------|
| `PacketType` | `StateUpdate` (0x01), `RenderRequest` (0x02), `RenderResponse` (0x03), `ControlAction` (0x04), `Heartbeat` (0x05) |
| `RenderTarget` | `Terminal`, `Telegram`, `Dashboard`, `GameEngine { engine }`, `Voice`, `JSON`, `A2A` |
| `ControlType` | `Button`, `Slider { min, max, value }`, `Toggle { on }`, `Text { placeholder }`, `Select { options, selected }` |
| `NotificationLevel` | `Info`, `Warning`, `Alert`, `Critical` |
| `DeadbandLevel` | `Green` (default), `Yellow`, `Red` |
| `A2UIError` | `Serialization(String)`, `InvalidPacket(String)`, `ChecksumMismatch`, `UnsupportedVersion(u8)`, `UnknownTarget(String)` |

### Key Methods

#### `UIState`
- `serialize(&self) -> Result<Vec<u8>, A2UIError>` — Bincode encoding.
- `deserialize(data: &[u8]) -> Result<Self, A2UIError>` — Bincode decoding.

#### `A2UIPacket`
- `new(packet_type, payload) -> Self` — Create packet with auto-computed CRC32.
- `encode(&self) -> Vec<u8>` — Produce wire bytes: `[ver:1][type:1][len:4 LE][payload:N][crc:4 LE]`.
- `decode(data: &[u8]) -> Result<Self, A2UIError>` — Parse wire bytes with full validation.

#### `crc32(data: &[u8]) -> u32` (public function)
- IEEE polynomial CRC32 with compile-time generated 256-entry lookup table.

### Helper Functions
- `sample_ui_state() -> UIState` — Build a rich sample state (2 rooms, 2 agents, 4 notifications, 5 controls).
- `sample_render_request(state) -> RenderRequest` — Build a sample render request for terminal target.

## How It Works

### Wire Format

Each packet is laid out as:
```
Offset  Size   Field
0       1      Protocol version (currently 1)
1       1      Packet type byte
2       4      Payload length (little-endian u32)
6       N      Payload (bincode-serialized data)
6+N     4      CRC32 checksum (little-endian u32)
```

Minimum packet size: 10 bytes (header + empty payload + checksum).

### CRC32 Implementation

Uses the IEEE 802.3 polynomial `0xEDB88320` (reversed representation). The lookup table is generated at compile time by a `const fn`:

```rust
const fn generate_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0u32;
    while i < 256 {
        let mut crc = i;
        let mut j = 0;
        while j < 8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB88320 } else { crc >> 1 };
            j += 1;
        }
        table[i as usize] = crc;
        i += 1;
    }
    table
}
```

Initial CRC value: `0xFFFFFFFF`, final XOR: `!crc` (standard CRC32/ISO-HDLC).

### Decode Validation

The decoder performs four checks in order:
1. **Length**: minimum 10 bytes
2. **Version**: must equal `PROTOCOL_VERSION` (currently 1)
3. **Payload length**: `data.len() >= 6 + payload_len + 4`
4. **Checksum**: recomputed CRC32 must match stored value

Each failure returns a specific `A2UIError` variant.

### Packet Type Encoding

| Packet Type | Byte Value |
|-------------|-----------|
| `StateUpdate` | `0x01` |
| `RenderRequest` | `0x02` |
| `RenderResponse` | `0x03` |
| `ControlAction` | `0x04` |
| `Heartbeat` | `0x05` |

Unknown type bytes (anything else) return `A2UIError::InvalidPacket`.

### Serialization

All state types use **bincode** for compact, zero-overhead binary encoding via serde. There's no schema overhead — the Rust types are self-describing. This makes packets significantly smaller than JSON for the same data.

### Room Controls

Rooms contain interactive `Control` widgets with 5 types:
- **Button** — simple action trigger
- **Slider** — continuous value with min/max bounds
- **Toggle** — on/off switch
- **Text** — text input with placeholder
- **Select** — dropdown with options and optional selection

### Gravity Vector

Each room has a `gravity: [f64; 3]` field representing three dimensions of room "pull":
- `gravity[0]` — attention weight
- `gravity[1]` — energy concentration
- `gravity[2]` — novelty factor

## The Math

### CRC32 Error Detection Properties

CRC32 with the IEEE polynomial provides these guarantees:
- All **single-bit** errors detected
- All **double-bit** errors detected (within a 2³²−1 bit window)
- All **odd numbers** of bit errors detected
- All **burst errors** ≤ 32 bits detected
- Probability of undetected random error ≈ 2⁻³²

### Packet Size Calculation

```
total_size = 1 (version) + 1 (type) + 4 (length) + N (payload) + 4 (checksum)
           = 10 + N bytes
```

For a typical `UIState` with 2 rooms, 2 agents, and notifications, the bincode payload is ~500 bytes, making the total packet ~510 bytes.

## Testing

**51 tests** covering:
- `UIState` bincode round-trips (full state, empty collections, garbage input)
- All enum variant round-trips: `DeadbandLevel` (default is Green), `ControlType` (all 5 variants), `NotificationLevel` (all 4), `RenderTarget` (all 7 including `GameEngine`), `PacketType` (all 5)
- `RenderRequest` round-trips for all target types
- `Viewport` with and without depth
- `RenderOptions` round-trip
- Packet encode/decode for all packet types
- Empty payload (10-byte packet) and large payload (40KB) round-trips
- Error cases: too-short input, bad version, checksum mismatch, truncated payload, unknown type byte
- `RoomState` with and without `wiki_summary`
- `AgentState` all levels (1–5)
- `Notification` with and without action
- Control interaction simulation (button press, slider bounds, toggle state, select options)
- Packet with embedded `UIState` payload (double-serialization round-trip)
- Packet with embedded `RenderRequest` payload
- CRC32 correctness: `crc32("123456789") == 0xCBF43926`, `crc32("") == 0x00000000`
- `A2UIError` display formatting
- Full pipeline: state → serialize → packet → encode → wire → decode → deserialize → state
- Full pipeline for Telegram and GameEngine targets
- Multiple rooms in a single state
- 50+ notifications in a single state

```bash
cargo test    # Run all 51 tests
```

## License

MIT
