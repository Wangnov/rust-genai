use rust_genai::types::content::{
    Content, FunctionResponse, FunctionResponseBlob, FunctionResponsePart, Part, Role,
};

fn main() {
    // 模拟工具返回多模态内容（例如图表图片）。
    let response = FunctionResponse {
        will_continue: None,
        scheduling: None,
        parts: Some(vec![FunctionResponsePart {
            inline_data: Some(FunctionResponseBlob {
                mime_type: "image/png".into(),
                data: vec![137, 80, 78, 71], // PNG 头部示例（替换为真实数据）
                display_name: Some("chart.png".into()),
            }),
            file_data: None,
        }]),
        id: Some("fn-1".into()),
        name: Some("render_chart".into()),
        response: None,
    };

    let content = Content::from_parts(vec![Part::function_response(response)], Role::Function);
    println!("{:?}", content);
}
