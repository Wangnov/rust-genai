use rust_genai::types::grounding::{
    GroundingChunk, GroundingMetadata, GroundingSupport, Segment, WebChunk,
};

fn main() {
    let mut metadata = GroundingMetadata::default();
    metadata.grounding_chunks.push(GroundingChunk::Web {
        web: WebChunk {
            uri: "https://example.com".into(),
            title: "Example".into(),
        },
    });
    metadata.grounding_supports.push(GroundingSupport {
        grounding_chunk_indices: vec![0],
        confidence_scores: vec![0.9],
        segment: Segment {
            part_index: 0,
            start_index: 0,
            end_index: 5,
            text: "Hello".into(),
        },
    });

    let text = "Hello world";
    let with_citations = metadata.add_citations(text);
    println!("{with_citations}");
    println!("{:?}", metadata.citation_uris());
}
