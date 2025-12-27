use rust_genai::types::operations::Operation;
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let op_name = match std::env::var("GENAI_OPERATION_NAME") {
        Ok(value) => value,
        Err(_) => {
            println!("set GENAI_OPERATION_NAME to wait for an operation.");
            return Ok(());
        }
    };
    let op = Operation {
        name: Some(op_name),
        ..Default::default()
    };
    let op = client.operations().wait(op).await?;
    println!("done: {:?}", op.done);
    Ok(())
}
