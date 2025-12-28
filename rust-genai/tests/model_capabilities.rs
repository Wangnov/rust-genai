use rust_genai::model_capabilities::{
    validate_code_execution_image_inputs, validate_function_response_media,
};
use rust_genai::types::content::{
    Content, FunctionResponse, FunctionResponseBlob, FunctionResponsePart, Part, Role,
};
use rust_genai::types::tool::{CodeExecution, Tool};
use serde_json::json;

#[test]
fn function_response_media_allowed_on_gemini3() {
    let response = FunctionResponse {
        will_continue: None,
        scheduling: None,
        parts: Some(vec![FunctionResponsePart {
            inline_data: Some(FunctionResponseBlob {
                mime_type: "image/png".to_string(),
                data: vec![1, 2, 3],
                display_name: None,
            }),
            file_data: None,
        }]),
        id: Some("id".to_string()),
        name: Some("fn".to_string()),
        response: Some(json!({"ok": true})),
    };
    let content = Content::from_parts(vec![Part::function_response(response)], Role::Model);
    let result = validate_function_response_media("gemini-3", &[content]);
    assert!(result.is_ok());
}

#[test]
fn code_execution_image_inputs_file_data_ok() {
    let tool = Tool {
        code_execution: Some(CodeExecution {}),
        ..Default::default()
    };
    let content = Content::from_parts(vec![Part::file_data("files/abc", "image/png")], Role::User);
    let result = validate_code_execution_image_inputs("gemini-3", &[content], Some(&[tool]));
    assert!(result.is_ok());
}

#[test]
fn code_execution_image_inputs_no_image_ok() {
    let tool = Tool {
        code_execution: Some(CodeExecution {}),
        ..Default::default()
    };
    let content = Content::from_parts(vec![Part::text("hello")], Role::User);
    let result = validate_code_execution_image_inputs("gemini-3", &[content], Some(&[tool]));
    assert!(result.is_ok());
}
