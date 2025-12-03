# Rust Echo Reply Server and Client

TCP and UDP echo server and client implementation written in Rust.

## Installation

### Install from GitHub

```bash
cargo install --git https://github.com/dimalinux/rust-echo-reply
```

### Install from Local Source

```bash
git clone https://github.com/dimalinux/rust-echo-reply
cd rust-echo-reply
cargo install --path .
```

The two binaries:
- `echo-server` - Echo server supporting TCP and/or UDP
- `echo-client` - Echo client for TCP or UDP

## Usage

```
echo-server (tcp|udp|both) [-b|--bind-addr <ADDR>]
echo-client (tcp|udp) [-s|--server-addr <ADDR>]
```

Default address: `127.0.0.1:2048`

## Quick Start

**Terminal 1 - Start the server:**
```bash
# Start both TCP and UDP servers on default port 2048
echo-server both
```

**Terminal 2 - Connect with TCP client:**
```bash
echo-client tcp
Connected to 127.0.0.1:2048 TCP
Enter text, newlines separate echo messages, control-d to quit.
Hello, World!
ECHO: Hello, World!
This is a test
ECHO: This is a test
^D
```

**Terminal 3 - Connect with UDP client:**
```bash
echo-client udp
Echo destination: 127.0.0.1:2048 UDP
Enter text, newlines separate echo messages, control-d to quit.
UDP message test
ECHO: UDP message test
^D
```

### Debug Logging

Use `RUST_LOG` environment variable to control logging. The default
level is `info`. For more verbose output, set it to `trace`.

```bash
# Trace-level logging (verbose)
RUST_LOG=trace echo-server both
```
