use rust_genai::types::models::{Image, ProductImage, RecontextImageConfig, RecontextImageSource};
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    // Recontext Image 仅支持 Vertex AI 后端。
    let client = Client::from_env()?;

    let source = RecontextImageSource {
        prompt: Some("studio product photo on a marble table".to_string()),
        person_image: None,
        product_images: Some(vec![ProductImage {
            product_image: Some(Image {
                gcs_uri: Some("gs://your-bucket/product.jpg".to_string()),
                ..Default::default()
            }),
        }]),
    };

    let response = client
        .models()
        .recontext_image(
            "imagen-product-recontext-preview-06-30",
            source,
            RecontextImageConfig::default(),
        )
        .await?;

    println!("images: {}", response.generated_images.len());
    Ok(())
}
