use std::fmt::{Debug, Display};

#[doc(hidden)]
pub use postcard as __exect_postcard;

#[doc(hidden)]
pub use inventory as __exect_inventory;

#[doc(hidden)]
pub use serde as __exect_serde;
use thiserror::Error;

pub trait Instruction: Debug + Display {
    fn get_code(&self) -> u8;
    fn to_bytes(self) -> Vec<u8>;
    fn execute(self: Box<Self>) -> Option<ExecutorSignal>;
}

#[derive(Debug)]
pub struct InstructionInfo {
    pub code: u8,
    pub name: &'static str,
    pub instruction_factory: fn(buffer: &[u8]) -> (Box<dyn Instruction>, &[u8]),
}

inventory::collect!(InstructionInfo);

pub fn get_instruction<T>(code: T) -> Option<&'static InstructionInfo>
where
    T: Into<u8>,
{
    let code: u8 = code.into();
    inventory::iter::<InstructionInfo>().find(|info| info.code == code)
}

#[derive(Debug)]
pub enum ManifestFunctionNameFormat {
    Original,
    UpperCamelCase,
    LowerCamelCase,
}

#[derive(Debug)]
pub enum ManifestOrdering {
    CodeFirst,
    NameFirst,
}

fn to_upper_camel_case(s: &str) -> String {
    let mut name = String::new();
    for (_i, word) in s.split('_').enumerate() {
        name.push_str(&word[0..1].to_uppercase());
        name.push_str(&word[1..]);
    }
    name
}

fn to_lower_camel_case(s: &str) -> String {
    let mut name = String::new();
    for (i, word) in s.split('_').enumerate() {
        if i == 0 {
            name.push_str(&word.to_lowercase());
        } else {
            name.push_str(&word[0..1].to_uppercase());
            name.push_str(&word[1..]);
        }
    }
    name
}

pub fn generate_manifest(
    version: i32,
    name: String,
    format: ManifestFunctionNameFormat,
    ordering: ManifestOrdering,
) -> String {
    let mut manifest = String::new();
    manifest.push_str(&format!("Version: {}\n", version));
    manifest.push_str(&format!("Name: {}\n", name));
    manifest.push_str(&format!("Ordering: {:?}\n", ordering));
    manifest.push_str("Instructions:\n");
    for info in inventory::iter::<InstructionInfo>() {
        let name = match format {
            ManifestFunctionNameFormat::Original => info.name,
            ManifestFunctionNameFormat::UpperCamelCase => &to_upper_camel_case(info.name),
            ManifestFunctionNameFormat::LowerCamelCase => &to_lower_camel_case(info.name),
        };
        let code = info.code;
        let instruction_name = format!("0x{:02X}", code);
        match ordering {
            ManifestOrdering::CodeFirst => {
                manifest.push_str(&format!("{} => {}\n", instruction_name, name));
            }
            ManifestOrdering::NameFirst => {
                manifest.push_str(&format!("{} => {}\n", name, instruction_name));
            }
        }
    }
    manifest
}

#[derive(Error, Debug)]
pub enum ExectError {
    #[error("Cannot execute, unknown instruction: {0}")]
    UnknownInstruction(u8),
}

pub struct BinaryBuilder {
    pub buffer: Vec<u8>,
}

impl BinaryBuilder {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn add(mut self, instruction: impl Instruction) -> Self {
        let code = instruction.get_code();
        let bytes = instruction.to_bytes();
        self.buffer.push(code);
        self.buffer.extend(bytes);
        self
    }

    pub fn get_address_for_next_instruction(&self) -> usize {
        self.buffer.len()
    }

    pub fn build(self) -> Vec<u8> {
        self.buffer
    }
}

pub enum ExecutorSignal {
    JumpTo(usize),
    Abort,
}

pub struct BinaryExecutor<'a> {
    pub buffer: &'a [u8],
}

impl<'a> BinaryExecutor<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer }
    }

    pub fn execute(&self) -> Result<(), ExectError> {
        let mut buffer = self.buffer;
        while !buffer.is_empty() {
            let code = buffer[0];
            if let Some(instruction_info) = get_instruction(code) {
                let (instruction, remaining_buffer) =
                    (instruction_info.instruction_factory)(&buffer[1..]);
                if let Some(signal) = instruction.execute() {
                    match signal {
                        ExecutorSignal::JumpTo(offset) => {
                            buffer = &self.buffer[offset..];
                            continue;
                        }
                        ExecutorSignal::Abort => return Ok(()),
                    }
                }
                buffer = remaining_buffer;
            } else {
                return Err(ExectError::UnknownInstruction(code));
            }
        }
        Ok(())
    }

    pub fn jump(&mut self, offset: usize) {
        self.buffer = &self.buffer[offset..];
    }
}
