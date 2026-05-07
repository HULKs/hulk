mod add_two_ints;

use add_two_ints::{AddTwoInts, AddTwoIntsResponse};
use ros_z::{Result, context::ContextBuilder};

#[tokio::main]
async fn main() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("add_two_ints_server").build().await?;
    let mut service_server = node
        .create_service_server::<AddTwoInts>("add_two_ints")
        .build()
        .await?;

    println!("AddTwoInts service server started, waiting for requests...");
    loop {
        let request = service_server.take_request_async().await?;
        let response = AddTwoIntsResponse {
            sum: request.message().a + request.message().b,
        };
        println!(
            "{} + {} = {}",
            request.message().a,
            request.message().b,
            response.sum
        );
        request.reply_async(&response).await?;
    }
}
