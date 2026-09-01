use fuzzy_parser_wasm_evaluation::{NativeRequest, native_oracle};
use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("read request");
    let request: NativeRequest = serde_json::from_str(&input).expect("valid native request");
    println!(
        "{}",
        serde_json::to_string(&native_oracle(request)).expect("serialize response")
    );
}
