# p2p-blockchain-rust

A minimal blockchain implementation in Rust demonstrating **peer-to-peer networking**,
**Proof of Work consensus**, and **longest-chain synchronization** across multiple nodes.

Built with [libp2p](https://libp2p.io/) (Gossipsub + mDNS) and [tokio](https://tokio.rs/).

---

## Features

- **Proof of Work (PoW)** — SHA-256 based mining with configurable difficulty
- **P2P Network** — nodes discover each other automatically via mDNS on the local network
- **Chain Synchronization** — new nodes sync with the longest valid chain on join
- **Longest-chain rule** — in case of conflict, the longest valid chain wins
- **Block validation** — every received block is validated before being added

---

## Architecture

Each node runs two phases:

**Phase 1 - Discovery**: On startup, the node listens for peers via mDNS for 1 second.
If peers are found, it requests their chain and adopts the longest valid one.

**Phase 2 - Main loop**: The node handles CLI commands and incoming network events concurrently.

Each node contains a local blockchain and a networking layer built on libp2p.
The event loop drives both stdin commands and swarm events using tokio `select!`.
Nodes communicate via three message types:

- `NewBlock` — broadcast a newly mined block to all peers
- `ChainRequest` — request the full chain from peers
- `ChainResponse` — respond with the local chain for synchronization

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable)

### Clone and Run
```bash
git clone https://github.com/AnTzikas/p2p-blockchain-rust.git
cd p2p-blockchain-rust
```

Start a node:
```bash
RUST_LOG=info cargo run
```

Start additional nodes in separate terminals - they will discover each other automatically via mDNS:
```bash
RUST_LOG=info cargo run
```

---
## Running with Docker

If you don't have Rust installed, you can run nodes using Docker.

**Build the Image:**
```bash
docker build -t p2p-blockchain-rust .
```

**Start a peer node:**
```bash
docker run --rm -it --network host p2p-blockchain-rust
```
---

## CLI Commands

| Command | Description |
|---|---|
| `ls p` | List connected peers |
| `add block <data>` | Mine and broadcast a new block |
| `ls chain` | Print the local blockchain |

---

## Demo: Longest-Chain Consensus

This demo shows the longest-chain rule in action across two nodes.

1. Start **Node A** and **Node B** in separate terminals
2. Wait for them to discover each other (`New peer discovered` in logs)
3. On **Node A**, run: `add block hello`
4. Immediately on **Node B** (within the 2s broadcast delay), run: `add block world`
5. Both nodes now have conflicting chains of equal length
6. On **Node A**, run: `add block tiebreaker`
7. Node B will receive the longer chain and replace its local one

Note: the 2-second delay before broadcasting is intentional. It creates a window
to manually simulate concurrent block creation across peers, demonstrating the
longest-chain consensus rule.

---

## Running Tests
```bash
cargo test
```

Tests cover:

- Genesis block creation
- Block validation (invalid previous hash rejection)
- Chain validity checking
- Tampered chain detection
- Chain restore from blocks

---

## Documentation

Full API docs generated via `cargo doc`:
```bash
cargo doc --open
```

---

## Tech Stack

| Technology | Role |
|---|---|
| Rust | Core implementation |
| libp2p | P2P networking (Gossipsub + mDNS) |
| tokio | Async runtime |
| serde / serde_json | Block and message serialization |
| ring / sha2 | SHA-256 hashing |
| num-bigint | PoW target comparison |