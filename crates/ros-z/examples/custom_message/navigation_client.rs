mod navigation_types;

use navigation_types::{NavigateTo, NavigateToRequest};
use ros_z::{Result, context::ContextBuilder};

#[tokio::main]
async fn main() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("navigation_client").build().await?;
    let service_client = node
        .create_service_client::<NavigateTo>("navigate_to")
        .build()
        .await?;

    let request = NavigateToRequest {
        target_x: 10.0,
        target_y: 20.0,
        max_speed: 1.5,
    };
    println!(
        "Sending navigation request: target=({:.1}, {:.1}), max_speed={:.1}",
        request.target_x, request.target_y, request.max_speed
    );
    let response = service_client.call_async(&request).await?;
    println!("Received navigation response: {:?}", response);

    Ok(())
}
