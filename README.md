# LAU A2UI Protocol

**Agent-to-UI protocol** — the wire format that makes the same state render as a Unity scene, a Telegram message, a terminal dashboard, or a JSON API response.

## Wire Format

`A2UIPacket` layout:

```
[version: u8][type: u8][payload_len: u32 LE][payload: &[u8]][checksum: u32 LE]
```

## Core Types

| Type | Description |
|------|-------------|
| `UIState` | Canonical rendering-agnostic state |
| `RoomState` | Room with gravity, controls, deadband |
| `AgentState` | Agent with level, energy, capabilities |
| `Control` | Button, Slider, Toggle, Text, Select |
| `Notification` | Info/Warning/Alert/Critical messages |
| `SystemMetrics` | Uptime, ticks, energy, response times |
| `RenderRequest` | State + target + viewport + options |
| `RenderTarget` | Terminal, Telegram, Dashboard, GameEngine, Voice, JSON, A2A |
| `A2UIPacket` | Versioned wire packet with CRC32 |

## Usage

```rust
use lau_a2ui_protocol::*;

let state = sample_ui_state();
let payload = state.serialize().unwrap();
let packet = A2UIPacket::new(PacketType::StateUpdate, payload);
let wire = packet.encode();

// On the other end:
let decoded = A2UIPacket::decode(&wire).unwrap();
let state = UIState::deserialize(&decoded.payload).unwrap();
```

## License

MIT
