# **mavshark** 🦈

_A lightweight CLI tool for listening to MAVLink messages over various connection types._

## **Installation**

### **1. Install via Cargo (Recommended)**

```sh
cargo install --path .
```

Or, after publishing to crates.io:

```sh
cargo install mavshark
```

### **2. Build & Run Locally**

```sh
cargo build --release
./target/release/mavshark listen udpin:0.0.0.0:14550 --time 60
```

### **3. Move Binary for Global Use**

```sh
sudo mv target/release/mavshark /usr/local/bin/
```

---

## **Usage**

### **Basic Command**

```sh
mavshark listen <ADDRESS> [OPTIONS]
```

### **Supported Connection Types**

| Type          | Format                     | Description                          |
| ------------- | -------------------------- | ------------------------------------ |
| TCP Server    | `tcpin:<addr>:<port>`      | Listens for incoming TCP connections |
| TCP Client    | `tcpout:<addr>:<port>`     | Connects to a TCP server             |
| UDP Server    | `udpin:<addr>:<port>`      | Listens for incoming UDP packets     |
| UDP Client    | `udpout:<addr>:<port>`     | Sends packets to a UDP server        |
| UDP Broadcast | `udpbcast:<addr>:<port>`   | Broadcasts UDP messages              |
| Serial        | `serial:<port>:<baudrate>` | Connects via a serial port           |
| File          | `file:<path>`              | Reads MAVLink messages from a file   |

### **Example Usage**

```sh
mavshark listen udpin:0.0.0.0:14550 --time 120 --system-id 1
```

### **Available Options**

| Option                | Short | Description                       |
| --------------------- | ----- | --------------------------------- |
| `--time <seconds>`    | `-t`  | Duration to listen (default: 60s) |
| `--system-id <id>`    |       | Filter by sender system ID        |
| `--component-id <id>` |       | Filter by component ID            |

---

## **Development**

### **Build**

```sh
cargo build --release
```

### **Run**

```sh
cargo run -- listen udpin:0.0.0.0:14550 --time 30
```

### **Test**

```sh
cargo test
```

---

## **License**

MIT License. See [LICENSE](LICENSE) for details.
