use exect_core::{
    BinaryBuilder, BinaryExecutor, ExecutorSignal, ManifestFunctionNameFormat, ManifestOrdering,
    generate_manifest,
};
use exect_macros::exect;

fn main() {
    println!(
        "{}",
        generate_manifest(
            3,
            "Main".to_string(),
            ManifestFunctionNameFormat::Original,
            ManifestOrdering::CodeFirst
        )
    );

    let binary = BinaryBuilder::new()
        .add(TestFunctionInstruction {
            message: "Hello1".to_string(),
        })
        .add(FooInstruction { input1: 42 })
        .build();

    println!("Binary: {:?}", binary);

    match BinaryExecutor::new(&binary).execute() {
        Ok(_) => println!("Execution successful"),
        Err(e) => println!("Execution failed: {:?}", e),
    }
}

#[exect(0x01)]
fn foo(input1: i32) {
    println!("foo: {input1}");
}

#[exect(0x02)]
fn test_function(message: String) {
    println!("message: {message}");
}

#[exect(0x03)]
fn jump_to(offset: usize) -> ExecutorSignal {
    println!("Jumping to offset: {offset}");
    ExecutorSignal::JumpTo(offset)
}
