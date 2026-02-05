//! Watch command implementations.

use color_eyre::Result;
use hulkz::{GraphEvent, Session};

/// Watches for node join/leave events.
pub async fn nodes(namespace: &str) -> Result<()> {
    let session = Session::create(namespace).await?;

    let (mut watcher, driver) = session.graph().nodes().watch().await?;
    tokio::spawn(driver);

    println!("Watching for node events in namespace: {}", namespace);
    println!("(Press Ctrl+C to exit)");
    println!();

    while let Some(event) = watcher.recv().await {
        match &event {
            GraphEvent::Joined(info) => {
                println!("Node joined: {}:{}", info.namespace, info.name);
            }
            GraphEvent::Left(info) => {
                println!("Node left: {}:{}", info.namespace, info.name);
            }
        }
    }

    Ok(())
}

/// Watches for publisher advertise/unadvertise events.
pub async fn publishers(namespace: &str) -> Result<()> {
    let session = Session::create(namespace).await?;

    let (mut watcher, driver) = session.graph().publishers().watch().await?;
    tokio::spawn(driver);

    println!("Watching for publisher events in namespace: {}", namespace);
    println!("(Press Ctrl+C to exit)");
    println!();

    while let Some(event) = watcher.recv().await {
        match &event {
            GraphEvent::Joined(info) => {
                println!(
                    "Publisher advertised: namespace={} node={} scope={} path={}",
                    info.namespace, info.node, info.scope, info.path
                );
            }
            GraphEvent::Left(info) => {
                println!(
                    "Publisher unadvertised: namespace={} node={} scope={} path={}",
                    info.namespace, info.node, info.scope, info.path
                );
            }
        }
    }

    Ok(())
}

/// Watches for session join/leave events.
pub async fn sessions(namespace: &str) -> Result<()> {
    let session = Session::create(namespace).await?;

    let (mut watcher, driver) = session.graph().sessions().watch().await?;
    tokio::spawn(driver);

    println!("Watching for session events in namespace: {}", namespace);
    println!("(Press Ctrl+C to exit)");
    println!();

    while let Some(event) = watcher.recv().await {
        match &event {
            GraphEvent::Joined(info) => {
                println!("Session joined: {}", info.id);
            }
            GraphEvent::Left(info) => {
                println!("Session left: {}", info.id);
            }
        }
    }

    Ok(())
}
