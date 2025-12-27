use rust_genai::computer_use::computer_use_function_declarations;

fn main() {
    let tools = computer_use_function_declarations();
    println!("computer use actions: {}", tools.len());
}
