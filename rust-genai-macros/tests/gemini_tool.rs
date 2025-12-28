use rust_genai_macros::GeminiTool;
use rust_genai_types::content::FunctionCall;
use rust_genai_types::enums::Type;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize, GeminiTool)]
#[gemini(name = "get_weather", description = "Get weather information.")]
struct GetWeather {
    /// City name
    city: String,
    /// Temperature unit
    #[gemini(optional, enum_values = "celsius,fahrenheit")]
    unit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, GeminiTool)]
/// Documentation only description.
struct MixedTool {
    #[gemini(rename = "q")]
    query: String,
    #[gemini(optional)]
    optional_flag: String,
    #[gemini(required)]
    maybe: Option<String>,
    #[gemini(skip)]
    hidden: String,
}

#[test]
fn test_gemini_tool_macro_schema() {
    let tool = GetWeather::as_tool();
    let declarations = tool.function_declarations.expect("missing declarations");
    let declaration = &declarations[0];
    assert_eq!(declaration.name, "get_weather");
    assert_eq!(
        declaration.description.as_deref(),
        Some("Get weather information.")
    );

    let schema = declaration.parameters.as_ref().expect("missing schema");
    assert_eq!(schema.ty, Some(Type::Object));
    let properties = schema.properties.as_ref().expect("missing properties");
    assert!(properties.contains_key("city"));
    assert!(properties.contains_key("unit"));

    let required = schema.required.as_ref().expect("missing required list");
    assert!(required.contains(&"city".to_string()));
    assert!(!required.contains(&"unit".to_string()));

    let unit_schema = properties.get("unit").expect("missing unit schema");
    assert_eq!(unit_schema.enum_values.as_ref().unwrap().len(), 2);
    assert_eq!(unit_schema.nullable, Some(true));
}

#[test]
fn test_gemini_tool_from_call() {
    let call = FunctionCall {
        id: None,
        name: Some("get_weather".to_string()),
        args: Some(json!({"city": "Beijing", "unit": "celsius"})),
        partial_args: None,
        will_continue: None,
    };

    let parsed = GetWeather::from_call(&call).expect("failed to parse");
    assert_eq!(parsed.city, "Beijing");
    assert_eq!(parsed.unit.as_deref(), Some("celsius"));
}

#[test]
fn test_gemini_tool_schema_with_skip_and_required() {
    let tool = MixedTool::as_tool();
    let declaration = tool
        .function_declarations
        .as_ref()
        .unwrap()
        .first()
        .unwrap();
    assert_eq!(declaration.name, "MixedTool");
    assert_eq!(
        declaration.description.as_deref(),
        Some("Documentation only description.")
    );
    let schema = declaration.parameters.as_ref().unwrap();
    let properties = schema.properties.as_ref().unwrap();
    assert!(properties.contains_key("q"));
    assert!(properties.contains_key("optional_flag"));
    assert!(properties.contains_key("maybe"));
    assert!(!properties.contains_key("hidden"));

    let required = schema.required.as_ref().unwrap();
    assert!(required.contains(&"q".to_string()));
    assert!(required.contains(&"maybe".to_string()));
    assert!(!required.contains(&"optional_flag".to_string()));
}

#[test]
fn test_gemini_tool_from_call_errors() {
    let call = FunctionCall {
        id: None,
        name: Some("OtherTool".to_string()),
        args: Some(json!({"q": "value", "optional_flag": "x"})),
        partial_args: None,
        will_continue: None,
    };
    let err = MixedTool::from_call(&call).unwrap_err();
    assert!(format!("{err}").contains("Expected"));

    let call = FunctionCall {
        id: None,
        name: Some("MixedTool".to_string()),
        args: None,
        partial_args: None,
        will_continue: None,
    };
    assert!(MixedTool::from_call(&call).is_err());
}
