# **mavshark** 🦈

_A lightweight CLI tool for listening to MAVLink messages over various connection types._

## **Installation**

### **1. Install via Cargo **

```sh
cargo install --path .
```

Or

```sh
cargo install mavshark
```

## **Usage**

### **Basic Command**

```sh
mavshark listen <ADDRESS> [OPTIONS]
```

```sh
mavshark help
```

```sh
mavshark listen --help
```

## Clarifications

#### Why a heartbeat

Mavrouter will only route traffic with a header.system_id to a connection that is sending messages with that system_id. So sending the same heartbeat as a receiving system_id will allow for sniffing all their incoming messages. Also, if SnifferSysId is set in mavrouter and a connection sends a heartbeat with that system_id, that connection will receive all traffic.

#### Why output to binary

The mavlink connection can also be made on .bin files, all the messages are then read and parsed correctly. This enabled the replay functionality.

## **License**

MIT License. See [LICENSE](LICENSE) for details.
