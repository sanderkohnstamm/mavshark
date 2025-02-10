# **mavshark** 🦈

_A lightweight CLI tool for recording and replaying to MAVLink messages._

## **Installation**

### **1. Install via Cargo**

```sh
cargo install --path .
```

Or

```sh
cargo install mavshark
```

## **Usage**

### **Basic Commands**

```sh
mavshark record <ADDRESS> [OPTIONS]
```

```sh
mavshark help
```

```sh
mavshark record --help
```

### **Example usage**
To record messages towards the drone. With mavrouter sniffer id 233, explaination below.

```sh
mavshark record udpin:0.0.0.0:14550 -o output.txt -i 233 --include-system-id 1
```

and then (experimental)

```sh
mavshark replay udpin:0.0.0.0:14550 output.txt
```

## Clarifications

#### Why a heartbeat

Mavrouter will only route traffic with a header.system_id to a connection that is sending messages with that system_id. So sending the same heartbeat as a receiving system_id will allow for sniffing all their incoming messages. 

Note that all messages from the drone get sent towards the connection untill mavrouter correctly registers the connection as the drones group when sending heartbeats with the drones id. This takes some seconds. 

Also, if SnifferSysId is set in mavrouter and a connection sends a heartbeat with that system_id, that connection will receive all traffic for all system ids. This is the recommended way to listen to messages, as sending a heartbeat to mimic another system id might have unexpected side effects in mavrouter. This can be done as follows:
```sh
SnifferSysId=<ID>
```
under general in the .conf file or

```sh
--sniffer <ID>
```

in the command.

#### Why output to binary or .txt

The mavlink connection can also be made on .bin files, all the messages are then read and parsed correctly.
For the replay functionality we need or own json messadges with the timestamps. Use regular output file for this.

## **License**

MIT License. See [LICENSE](LICENSE) for details.
