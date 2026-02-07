# **MAVSHARK** 🦈

_A lightweight TUI tool for inspecting, recording, and replaying MAVLink messages._

## **Installation**

```sh
cargo install mavshark
```

Or build from source:

```sh
cargo install --path .
```

## **Usage**

### Live inspection

Connect and inspect MAVLink traffic in a terminal UI:

```sh
mavshark                                  # default: udpin:0.0.0.0:14550
mavshark udpout:127.0.0.1:14550
mavshark tcpout:127.0.0.1:5760
mavshark serial:/dev/ttyUSB0:57600
```

### Recording

Record messages to a JSON Lines file while inspecting:

```sh
mavshark --record flight.jsonl
mavshark --record flight.jsonl --record-filter HEARTBEAT,ATTITUDE
mavshark --record flight.jsonl --record-filter 0,30
```

`--record-filter` accepts comma-separated message names or numeric IDs. Omit it to record everything.

### Replay

Open a recorded file in the replay TUI:

```sh
mavshark replay flight.jsonl
```

### Heartbeat

Send heartbeats with a specific system ID so mavrouter routes traffic to your connection:

```sh
mavshark --heartbeat-sys-id 254
mavshark --heartbeat-sys-id 254 --heartbeat-comp-id 1
```

If `SnifferSysId` is configured in mavrouter, sending a heartbeat with that ID will receive all traffic across all system IDs.

## **Keybindings**

| Key | Action |
|---|---|
| `j` / `k` or arrows | Navigate messages |
| `/` | Filter by message name or sys\_id:comp\_id |
| `s` | Cycle sort mode (A-Z / Hz / Count) |
| `d` / `u` | Scroll detail pane down / up |
| `g` / `G` | Jump to first / last message (replay) |
| `q` / `Ctrl-c` | Quit |

## **Connection types**

Supported via [rust-mavlink](https://github.com/mavlink/rust-mavlink):

- `tcpin:<addr>:<port>` — TCP server (listen)
- `tcpout:<addr>:<port>` — TCP client
- `udpin:<addr>:<port>` — UDP server (listen)
- `udpout:<addr>:<port>` — UDP client
- `udpbcast:<addr>:<port>` — UDP broadcast
- `serial:<port>:<baudrate>` — Serial
- `file:<path>` — Read from file

## **License**

MIT License. See [LICENSE](LICENSE) for details.
