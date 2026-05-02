mod navigation_types;

use navigation_types::{NavigateTo, NavigateToResponse};
use ros_z::{Result, context::ContextBuilder};

#[tokio::main]
async fn main() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("navigation_server").build().await?;
    let mut service_server = node
        .create_service_server::<NavigateTo>("navigate_to")
        .build()
        .await?;

    println!("Navigation server ready, waiting for requests...");
    loop {
        let request = service_server.take_request_async().await?;
        let distance =
            (request.message().target_x.powi(2) + request.message().target_y.powi(2)).sqrt();
        let response = if request.message().max_speed > 0.0 && request.message().max_speed < 5.0 {
            NavigateToResponse {
                success: true,
                estimated_duration: distance / request.message().max_speed,
                message: format!("Path planned. Distance: {distance:.2}m"),
            }
        } else {
            NavigateToResponse {
                success: false,
                estimated_duration: 0.0,
                message: "max_speed must be between 0 and 5 m/s".to_string(),
            }
        };
        println!("Sending navigation response: {:?}", response);
        request.reply_async(&response).await?;
    }
}
