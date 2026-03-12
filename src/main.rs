use tokio::{io, io::AsyncBufReadExt, select, time::{sleep, timeout, Duration}};
use futures::stream::StreamExt;
use std::error::Error;


mod block;
mod blockchain;
mod util;
mod proof_of_work;
mod transaction;
mod networking;

use networking::{init_network, NetworkMessageData, broadcast_message, list_peers, handle_event, handle_phase1};
use blockchain::*;
use transaction::Transaction;
use block::Block;

/// Entry point for the Blockchain P2P node.
///
/// This function initializes the network, discovers peers, synchronizes
/// with existing chains, and starts the main event loop for user commands
/// and swarm events.
///
/// # Phases:
/// 1. **Peer Discovery (`Phase1`)**: Uses mDNS to discover peers and attempts to synchronize.
/// 2. **Main Event Loop (`Phase2`)**: Handles stdin commands and swarm events.
///
/// # Commands:
/// - `ls p` → List connected peers.
/// - `add block <data>` → Add a block with the provided data.
/// - `ls chain` → Print the current blockchain.
///
/// # Example Usage:
/// ```bash
/// cargo run
/// # Then in the prompt:
/// add block HelloWorld
/// ls chain
/// ls p
/// ```
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    // * Initialize P2P network and networking behaviour
    let (mut swarm, topic, peer_id) = init_network()?;
    let mut local_blockchain = Blockchain::new();

    /* 
    * Phase1: 
    * -> Run initial swarm event handler with timeout for peer discovery and sync with existing peers (if any)
    * -> If peers are discovered, sync with their chain (preferably the longest one)
    * -> If no peers are discovered after timeout, proceed with a new blockchain
    */
    println!("Node running. Phase1: mDNS discovery...");

    let sync_timeout = Duration::from_secs(1);
    let _sync_result = timeout(sync_timeout, handle_phase1(&mut swarm)).await;

    // Broadcast a ChainRequest to peers after initial discovery
    broadcast_message(&mut swarm, &topic, NetworkMessageData::ChainRequest);

    /*
    * Phase2:
    * -> Continue with the main event loop handling stdin commands and swarm events
    */
    println!("Phase2: Main event loop");
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    loop {
        select! {
            //Handle stdin commands
            Ok(Some(line)) = stdin.next_line() => {
                match line.as_str() {
                    //List connected peers
                    cmd if cmd.starts_with("ls p") => {
                        println!("My id: {:?}\n", peer_id);
                        list_peers(&swarm);
                    }

                    //Add a new block with user-provided data
                    cmd if cmd.starts_with("add block") => {
                        let data = cmd.strip_prefix("add block").unwrap_or("").trim();
                        if !data.is_empty() {
                            // Create the new transaction
                            let tx1 = Transaction::new(data.to_string());

                            // Create the new block
                            let Some(prev_block) = local_blockchain.get_last_block() else {
                                tracing::error!("Blockchain has no blocks — this should never happen");
                                continue;
                            };
                            let new_block = Block::new_block(
                                prev_block.get_hash().to_string(),
                                &[tx1],
                                prev_block.get_height() + 1,
                            );
                            // Add to local blockchain
                            local_blockchain.add_block(new_block.clone());

                            // Artificial delay
                            // Intentional delay to allow manual testing of concurrent block addition across peers.
                            // This gives time to add a block on another node simultaneously,
                            // demonstrating the longest-chain consensus rule.
                            sleep(Duration::from_secs(2)).await;

                            // Publish it to the network
                            let serialized_block = serde_json::to_string(&new_block).unwrap();
                            broadcast_message(&mut swarm, &topic, NetworkMessageData::NewBlock(serialized_block));


                            println!("Block added and broadcasted: {}", data);
                        }
                    }

                    // List the current blockchain
                    cmd if cmd.starts_with("ls chain") => {
                        println!("Current Blockchain:");
                        for block in local_blockchain.get_blocks() {
                            println!("---------------------------");
                            println!("Height: {}", block.get_height());
                            println!("Timestamp: {}", block.get_timestamp());
                            println!("Transactions: {:?}", block.get_transactions());
                            println!("Previous Hash: {}", block.get_pre_block_hash());
                            println!("Hash: {}", block.get_hash());
                            println!("Nonce: {}", block.get_nonce());
                        }
                    }

                    _ => println!("Unknown command"),
                }
            }

            //Handle swarm events (e.g., new peers, incoming messages)
            event = swarm.select_next_some() => handle_event(event, &mut swarm, &topic, &mut local_blockchain),
        }
    }
}
