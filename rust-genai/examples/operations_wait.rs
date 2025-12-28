use rust_genai::types::operations::Operation;
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let Ok(op_name) = std::env::var("GENAI_OPERATION_NAME") else {
        println!("set GENAI_OPERATION_NAME to wait for an operation.");
        return Ok(());
    };
    let op = Operation {
        name: Some(op_name),
        ..Default::default()
    };
    let op = client.operations().wait(op).await?;
    println!("done: {done:?}", done = op.done);
    Ok(())
}
