mod add_two_ints;

use std::time::Duration;

use add_two_ints::{AddTwoInts, AddTwoIntsRequest};
use ros_z::{Result, context::ContextBuilder};

#[tokio::main]
async fn main() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("add_two_ints_client").build().await?;
    let service_client = node
        .create_service_client::<AddTwoInts>("add_two_ints")
        .build()
        .await?;

    let request = AddTwoIntsRequest { a: 1, b: 2 };
    println!("Sending request: {} + {}", request.a, request.b);
    let response = service_client
        .call_with_timeout_async(&request, Duration::from_secs(5))
        .await?;
    println!("Received response: {}", response.sum);

    Ok(())
}
