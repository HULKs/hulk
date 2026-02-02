//! Discovery example
//!
//! This example demonstrates how to discover nodes and publishers in the network.
//!
//! Run with: `cargo run --example discovery`

use hulkz::{NodeEvent, PublisherEvent, Result, Session, SessionEvent};
use tokio::select;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a session
    let session = Session::create("demo").await?;
    println!("Session created: {}", session.id());
    println!("Discovering network...\n");

    // List current sessions (returns Vec<String> of session IDs)
    let sessions = session.list_sessions().await?;
    println!("Sessions ({}):", sessions.len());
    for s in &sessions {
        println!("  - {}", s);
    }

    // List current nodes (returns Vec<String> of node names)
    let nodes = session.list_nodes().await?;
    println!("\nNodes ({}):", nodes.len());
    for n in &nodes {
        println!("  - {}", n);
    }

    // List current publishers (returns Vec<PublisherInfo>)
    let publishers = session.list_publishers().await?;
    println!("\nPublishers ({}):", publishers.len());
    for p in &publishers {
        println!("  - {} on {} (scope: {:?})", p.path, p.node, p.scope);
    }

    println!("\n--- Watching for changes (Ctrl+C to stop) ---\n");

    // Watch for events (async methods that return (Watcher, driver))
    let (mut session_watcher, session_driver) = session.watch_sessions().await?;
    let (mut node_watcher, node_driver) = session.watch_nodes().await?;
    let (mut pub_watcher, pub_driver) = session.watch_publishers().await?;

    // Spawn the driver futures
    tokio::spawn(session_driver);
    tokio::spawn(node_driver);
    tokio::spawn(pub_driver);

    // Handle events
    loop {
        select! {
            Some(event) = session_watcher.recv() => {
                match event {
                    SessionEvent::Joined(id) => println!("[Session] Joined: {}", id),
                    SessionEvent::Left(id) => println!("[Session] Left: {}", id),
                }
            }
            Some(event) = node_watcher.recv() => {
                match event {
                    NodeEvent::Joined(name) => println!("[Node] Joined: {}", name),
                    NodeEvent::Left(name) => println!("[Node] Left: {}", name),
                }
            }
            Some(event) = pub_watcher.recv() => {
                match event {
                    PublisherEvent::Advertised(info) => {
                        println!("[Publisher] Advertised: {} on {} (scope: {:?})", info.path, info.node, info.scope)
                    }
                    PublisherEvent::Unadvertised(info) => {
                        println!("[Publisher] Unadvertised: {} on {}", info.path, info.node)
                    }
                }
            }
        }
    }
}
