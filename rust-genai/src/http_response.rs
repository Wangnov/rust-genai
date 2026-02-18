use std::collections::HashMap;

use reqwest::header::HeaderMap;

use rust_genai_types::http::HttpResponse;

pub(crate) fn sdk_http_response_from_headers(headers: &HeaderMap) -> HttpResponse {
    let mut map: HashMap<String, String> = HashMap::new();
    for (name, value) in headers.iter() {
        let Ok(value_str) = value.to_str() else {
            continue;
        };
        let key = name.as_str().to_string();
        map.entry(key)
            .and_modify(|existing| {
                if !existing.is_empty() {
                    existing.push_str(", ");
                }
                existing.push_str(value_str);
            })
            .or_insert_with(|| value_str.to_string());
    }

    HttpResponse {
        headers: Some(map),
        body: None,
    }
}

pub(crate) fn sdk_http_response_from_headers_and_body(
    headers: &HeaderMap,
    body: String,
) -> HttpResponse {
    let mut response = sdk_http_response_from_headers(headers);
    response.body = Some(body);
    response
}
