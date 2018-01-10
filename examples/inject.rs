extern crate sophon_wasm;

use std::env;

use sophon_wasm::elements;
use sophon_wasm::builder;

pub fn inject_nop(opcodes: &mut elements::Opcodes) {
    use sophon_wasm::elements::Opcode::*;
    let opcodes = opcodes.elements_mut();
    let mut position = 0;
    loop {
        let need_inject = match &opcodes[position] {
            &Block(_) | &If(_) => true,
            _ => false,
        };
        if need_inject {
            opcodes.insert(position + 1, Nop);
        }

        position += 1;
        if position >= opcodes.len() {
            break;
        }
    }
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        println!("Usage: {} input_file.wasm output_file.wasm", args[0]);
        return;
    }

    let mut module = sophon_wasm::deserialize_file(&args[1]).unwrap();

    for section in module.sections_mut() {
        match section {
            &mut elements::Section::Code(ref mut code_section) => {
                for ref mut func_body in code_section.bodies_mut() {
                    inject_nop(func_body.code_mut());
                }
            },
            _ => { }
        }
    }

    let mut build = builder::from_module(module);
    let import_sig = build.push_signature(
        builder::signature()
            .param().i32()
            .param().i32()
            .return_type().i32()
            .build_sig()
    );
    let build = build.import()
        .module("env")
        .field("log")
        .external().func(import_sig)
        .build();

    sophon_wasm::serialize_to_file(&args[2], build.build()).unwrap();
}