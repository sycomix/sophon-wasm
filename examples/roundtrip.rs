extern crate sophon_wasm;

use std::env;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        println!("Usage: {} in.wasm out.wasm", args[0]);
        return;
    }

    let module = sophon_wasm::deserialize_file(&args[1]).expect("Failed to load module");

    sophon_wasm::serialize_to_file(&args[2], module).expect("Failed to write module");
}
