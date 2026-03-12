use std::{
    collections::hash_map::DefaultHasher,
    error::Error,
    hash::{Hash, Hasher},
    time::Duration,
};

use libp2p::{
    gossipsub, mdns, noise,
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
    tcp, yamux,
    identity, PeerId,
};
use serde::{Serialize, Deserialize};
use tracing_subscriber::EnvFilter;
use std::collections::HashSet;
use futures::StreamExt;
use tracing::{info, error};


use crate::blockchain::Blockchain;
use crate::block::Block;
use crate::util::current_timestamp;

/// Represents the core network behavior combining Gossipsub and mDNS.
///
/// This structure manages the peer-to-peer communication, enabling block
/// broadcasting and peer discovery within the network.
///
/// # Components:
/// - **Gossipsub**: For broadcasting blocks and messages.
/// - **mDNS**: For local network peer discovery.
///
/// # Example
/// ```
/// use your_crate::CustomBehaviour;
/// use libp2p::gossipsub;
///
/// let gossipsub = gossipsub::Behaviour::new(...);
/// let mdns = mdns::tokio::Behaviour::new(...);
/// let behaviour = CustomBehaviour { gossipsub, mdns };
/// ```
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "CustomBehaviourEvent")]
pub struct CustomBehaviour {
    /// Gossipsub protocol for peer-to-peer message broadcasting.
    pub gossipsub: gossipsub::Behaviour,

    /// mDNS for local peer discovery.
    pub mdns: mdns::tokio::Behaviour,
}

/// Defines the events that the `CustomBehaviour` can emit.
///
/// This includes events from both the Gossipsub and mDNS protocols.
#[derive(Debug)]
pub enum CustomBehaviourEvent {
    /// Event triggered by Gossipsub (e.g., new messages or peer discovery).
    GossipSub(Box<gossipsub::Event>),

    /// Event triggered by mDNS for peer discovery and expiration.
    Mdns(mdns::Event),
}

impl From<gossipsub::Event> for CustomBehaviourEvent {
    /// Converts a Gossipsub event into a `CustomBehaviourEvent`.
    fn from(event: gossipsub::Event) -> Self {
        CustomBehaviourEvent::GossipSub(Box::new(event))
    }
}

impl From<mdns::Event> for CustomBehaviourEvent {
    /// Converts an mDNS event into a `CustomBehaviourEvent`.
    fn from(event: mdns::Event) -> Self {
        CustomBehaviourEvent::Mdns(event)
    }
}

/// Represents different types of network messages.
#[derive(Serialize, Deserialize, Debug)]
pub enum NetworkMessageData {
    /// Broadcasts a new block to the network.
    NewBlock(String),

    /// Requests the full blockchain from peers.
    ChainRequest,

    /// Responds to a chain request with the entire blockchain.
    ChainResponse(Vec<String>),
}

/// Represents a network message that includes a timestamp for deduplication and ordering.
#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkMessage {
    /// The timestamp when the message was created (in UNIX epoch seconds).
    pub timestamp: i64,

    /// The type of the network message (e.g., new block, chain request, chain response).
    pub message_type: NetworkMessageData,
}

impl NetworkMessage {
    /// Creates a new `NetworkMessage` with the current timestamp.
    ///
    /// # Arguments
    /// * `message_type` - The type of the network message.
    ///
    /// # Example
    /// ```
    /// let msg = NetworkMessage::new(NetworkMessageData::ChainRequest);
    /// ```
    pub fn new(message_type: NetworkMessageData) -> Self {
        NetworkMessage {
            timestamp: current_timestamp(),
            message_type,
        }
    }

}


/// Initializes the P2P network using libp2p with Gossipsub and mDNS protocols.
///
/// This function sets up the networking stack, initializes peer discovery,
/// and subscribes to the main Gossipsub topic (`"blockchain-net"`).
///
/// # Returns
///
/// - `Ok((Swarm<CustomBehaviour>, gossipsub::IdentTopic))` on success.
/// - `Err` if any part of the setup fails.
///
/// # Example
/// ```
/// let (mut swarm, topic) = init_network().unwrap();
/// broadcast_message(&mut swarm, &topic, NetworkMessage::ChainRequest);
/// ```
///
/// # Components
/// - **Gossipsub**: For broadcasting messages between peers.
/// - **mDNS**: For local peer discovery.
/// - **Noise Protocol**: For secure communication.
///
/// # Panics
/// - If the network setup fails or invalid configuration is provided.
pub fn init_network() -> Result<(Swarm<CustomBehaviour>, gossipsub::IdentTopic, PeerId), Box<dyn Error>> {
    // Initialize tracing/logging for better debugging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .ok();

    // Generate local peer identity
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    info!("Local peer id: {:?}", local_peer_id);

    // Message ID generation for deduplication
    let message_id_fn = |message: &gossipsub::Message| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        gossipsub::MessageId::from(s.finish().to_string())
    };

    // Gossipsub Configuration
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .message_id_fn(message_id_fn)
        .build()?;

    // Initialize Gossipsub behaviour
    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(local_key.clone()),
        gossipsub_config,
    )?;

    // mDNS Configuration for peer discovery
    let mdns_config = mdns::Config {
        enable_ipv6: false,
        ttl: Duration::from_secs(20),
        query_interval: Duration::from_secs(10),
    };

    let mdns = mdns::tokio::Behaviour::new(mdns_config, local_peer_id)?;

    // Combine behaviours into a single network behaviour
    let behaviour = CustomBehaviour { gossipsub, mdns };

    // Build the Swarm (the libp2p network stack)
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)?
        .with_behaviour(|_| Ok(behaviour))?
        .build();

    // Listen on a random TCP port
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // Subscribe to the main topic
    let topic = gossipsub::IdentTopic::new("blockchain-net");
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    Ok((swarm, topic, local_peer_id))
}

/// Broadcasts a message to all connected peers in the network.
///
/// Checks if there are connected peers before attempting to broadcast.
/// If no peers are found, it skips broadcasting.
///
/// # Arguments
///
/// - `swarm`: The active libp2p Swarm.
/// - `topic`: The Gossipsub topic to broadcast to.
/// - `msg`: The `NetworkMessage` to broadcast.
///
/// # Example
/// ```
/// broadcast_message(&mut swarm, &topic, NetworkMessage::NewBlock("Block Data".to_string()));
/// ```
pub fn broadcast_message(
    swarm: &mut Swarm<CustomBehaviour>,
    topic: &gossipsub::IdentTopic,
    msg: NetworkMessageData,
) {
    let connected_peers: Vec<_> = swarm.behaviour().mdns.discovered_nodes().collect();

    if connected_peers.is_empty() {
        info!("No peers connected. Message not broadcasted.");
        return;
    }
    let packet = NetworkMessage::new(msg);
    let data = serde_json::to_vec(&packet).unwrap();
    match swarm.behaviour_mut().gossipsub.publish(topic.clone(), data) {
        Ok(_) => info!("Message broadcasted."),
        Err(e) => error!("Failed to broadcast: {:?}", e),
    }
}

/// Lists all currently connected peers.
///
/// Retrieves peer IDs from mDNS and prints them to the console.
///
/// # Arguments
///
/// - `swarm`: The active libp2p Swarm.
///
/// # Example
/// ```
/// list_peers(&mut swarm);
/// ```
pub fn list_peers(swarm: &Swarm<CustomBehaviour>) {
    info!("Connected peers:");
    let peers: HashSet<_> = swarm.behaviour().mdns.discovered_nodes().collect();
    for peer in peers {
        info!("{:?}", peer);
    }
}

/// Handles incoming events from the libp2p swarm.
///
/// This function processes events related to:
/// - **Gossipsub messages** (e.g., new blocks, chain requests/responses)
/// - **mDNS peer discovery events**
///
/// # Arguments
/// - `event`: The incoming `SwarmEvent` (either Gossipsub or mDNS-related).
/// - `swarm`: The libp2p Swarm instance handling network events.
/// - `topic`: The Gossipsub topic for broadcasting messages.
/// - `local_blockchain`: The local blockchain instance to update based on received events.
///
/// # Behavior
/// - **NewBlock**: Adds a received block to the local chain after validation.
/// - **ChainRequest**: Responds with the entire blockchain to the requesting peer.
/// - **ChainResponse**: Replaces the local chain if the received chain is longer.
/// - **Peer Discovery**: Adds new peers to the mesh.
/// - **Peer Expiration**: Removes expired peers.
///
/// # Example
/// ```
/// handle_event(event, &mut swarm, &topic, &mut local_blockchain);
/// ```
pub fn handle_event(
    event: SwarmEvent<CustomBehaviourEvent>,
    swarm: &mut Swarm<CustomBehaviour>,
    topic: &gossipsub::IdentTopic,
    local_blockchain: &mut Blockchain,
) {
    match event {
        //Handle Gossipsub messages
        SwarmEvent::Behaviour(CustomBehaviourEvent::GossipSub(boxed)) => {
            if let gossipsub::Event::Message { message, .. } = *boxed {
                if let Ok(decoded) = serde_json::from_slice::<NetworkMessage>(&message.data) {
                    match decoded.message_type {
                        //Handle NewBlock message
                        NetworkMessageData::NewBlock(block_data) => {
                            info!("New Block Received: {:?}", block_data);

                            // Deserialize the received block data
                            let block: Block = match serde_json::from_str(&block_data) {
                                Ok(b) => b,
                                Err(e) => {
                                    error!("Failed to deserialize Block: {:?}", e);
                                    return;
                                }
                            };

                            // Add block after validation
                            if !local_blockchain.add_block(block) {
                                error!("Invalid block received. Requesting full chain for validation...");
                                broadcast_message(swarm, topic, NetworkMessageData::ChainRequest);
                                return;
                            }
                            info!("Successfully added the block to local blockchain!");
                        }

                        //Handle ChainRequest message
                        NetworkMessageData::ChainRequest => {
                            info!("Chain requested by peer {:?}", message.source);

                            // Serialize local blockchain and send as ChainResponse
                            let serialized_blocks: Vec<String> = local_blockchain.get_blocks()
                                .iter()
                                .map(|block| serde_json::to_string(block).unwrap())
                                .collect();

                            let response = NetworkMessageData::ChainResponse(serialized_blocks);
                            broadcast_message(swarm, topic, response);
                            
                        }

                        //Handle ChainResponse message
                        NetworkMessageData::ChainResponse(serialized_blocks) => {
                            let mut deserialized_chain = Vec::new();

                            // Deserialize the received chain
                            for block_str in serialized_blocks {
                                match serde_json::from_str::<Block>(&block_str) {
                                    Ok(block) => deserialized_chain.push(block),
                                    Err(e) => error!("Error deserializing block: {:?}", e),
                                }
                            }

                            // Replace local chain if received chain is longer
                            // if deserialized_chain.len() > local_blockchain.get_blocks().len() {
                            //     info!("Replacing local chain with received chain.");
                            //     *local_blockchain = Blockchain::from_blocks(deserialized_chain);
                            if deserialized_chain.len() > local_blockchain.get_blocks().len() {
                                let candidate = Blockchain::from_blocks(deserialized_chain);
                                if candidate.is_valid() {
                                    info!("Replacing local chain with longer valid chain.");
                                    *local_blockchain = candidate;
                                } else {
                                    error!("Received longer chain is invalid. Ignoring.");
                                }
                            } else if (deserialized_chain.len() == local_blockchain.get_blocks().len()) && deserialized_chain.len() == 1 {
                                info!("Synchronizing with existing chain!");
                                *local_blockchain = Blockchain::from_blocks(deserialized_chain);
                            } else {
                                info!("Received chain is not longer than the local chain. Ignoring.");
                            }
                        }
                    }
                } else {
                    error!("Received invalid message from {:?}", message.source);
                }
            }
        }

        //Handle new peer discovery via mDNS
        SwarmEvent::Behaviour(CustomBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
            for (peer_id, _) in peers {
                info!("New peer discovered: {:?}", peer_id);
                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
            }
        }

        //Handle peer expiration
        SwarmEvent::Behaviour(CustomBehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
            for (peer_id, _) in peers {
                info!("Peer expired: {:?}", peer_id);
                swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
            }
        }

        _ => {}
    }
}

/// Listens for network events and handles peer discovery.
///
/// This function runs an infinite loop that listens for new peers and prints
/// discovery events.
///
/// # Arguments
/// - `swarm`: The libp2p Swarm handling peer discovery and messaging.
///
/// # Example
/// ```
/// handle_phase1(&mut swarm).await;
/// ```
pub async fn handle_phase1(
    swarm: &mut Swarm<CustomBehaviour>,
) {
    loop {
        // if let Some(event) = swarm.next().await {
        //     if let SwarmEvent::Behaviour(CustomBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) = event {
        //         for (peer_id, _) in peers {
        //             info!("New peer discovered: {:?}", peer_id);
        //             swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
        //         }
        //     }
            
        // }
        if let Some(SwarmEvent::Behaviour(CustomBehaviourEvent::Mdns(mdns::Event::Discovered(peers)))) = swarm.next().await {
            for (peer_id, _) in peers {
                info!("New peer discovered: {:?}", peer_id);
                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
            }
        }
    }
}
