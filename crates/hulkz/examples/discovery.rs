//! Discovery example
//!
//! This example demonstrates how to discover nodes and publishers in the network.
//!
//! Run with: `cargo run --example discovery`

use hulkz::{GraphEvent, Result, Session};
use tokio::select;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a session
    let session = Session::create("demo").await?;
    println!("Session created: {}", session.id());
    println!("Discovering network...\n");

    // List current sessions using the fluent API
    let sessions = session.graph().sessions().list().await?;
    println!("Sessions ({}):", sessions.len());
    for s in &sessions {
        println!("  - {}", s);
    }

    // List current nodes using the fluent API
    let nodes = session.graph().nodes().list().await?;
    println!("\nNodes ({}):", nodes.len());
    for n in &nodes {
        println!("  - {}", n);
    }

    // List current publishers (returns Vec<PublisherInfo>)
    let publishers = session.graph().publishers().list().await?;
    println!("\nPublishers ({}):", publishers.len());
    for p in &publishers {
        println!("  - {} on {} (scope: {:?})", p.path, p.node, p.scope);
    }

    println!("\n--- Watching for changes (Ctrl+C to stop) ---\n");

    // Watch for events using the new fluent API
    let (mut session_watcher, session_driver) = session.graph().sessions().watch().await?;
    let (mut node_watcher, node_driver) = session.graph().nodes().watch().await?;
    let (mut pub_watcher, pub_driver) = session.graph().publishers().watch().await?;

    // Spawn the driver futures
    tokio::spawn(session_driver);
    tokio::spawn(node_driver);
    tokio::spawn(pub_driver);

    // Handle events using the new GraphEvent<T> enum
    loop {
        select! {
            Some(event) = session_watcher.recv() => {
                match event {
                    GraphEvent::Joined(info) => println!("[Session] Joined: {}", info.id),
                    GraphEvent::Left(info) => println!("[Session] Left: {}", info.id),
                }
            }
            Some(event) = node_watcher.recv() => {
                match event {
                    GraphEvent::Joined(info) => println!("[Node] Joined: {}", info.name),
                    GraphEvent::Left(info) => println!("[Node] Left: {}", info.name),
                }
            }
            Some(event) = pub_watcher.recv() => {
                match event {
                    GraphEvent::Joined(info) => {
                        println!("[Publisher] Advertised: {} on {} (scope: {:?})", info.path, info.node, info.scope)
                    }
                    GraphEvent::Left(info) => {
                        println!("[Publisher] Unadvertised: {} on {}", info.path, info.node)
                    }
                }
            }
        }
    }
}
