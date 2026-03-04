//! iot-flow
//! ========
//!
//! A DSL that turns sensor‑to‑actuator pipelines into safe,
//! zero‑runtime Rust/embedded C.
//!
//! The crate exposes a small, well‑documented API for parsing `.flow` files,
//! generating Rust code from the AST and executing the resulting pipeline in
//! a lightweight runtime.  It is designed to be used both as a library and
//! as a command‑line tool via `iot-flow-cli`.  
//!
//! ## Author
//! This project was initiated by **Myroslav Mokhammad Abdeljawwad**.

#![deny(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};

pub mod parser;
pub mod codegen;
pub mod runtime;

/// Errors that can occur while working with iot‑flow pipelines.
#[derive(Debug)]
pub enum Error {
    /// IO error when reading or writing files
    Io(std::io::Error),
    /// Parser error
    Parse(parser::ParseError),
    /// Code generation error
    Generate(codegen::GenerateError),
    /// Runtime execution error
    Runtime(runtime::RuntimeError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::Parse(e) => write!(f, "Parse error: {}", e),
            Error::Generate(e) => write!(f, "Code generation error: {}", e),
            Error::Runtime(e) => write!(f, "Runtime error: {}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self { Error::Io(err) }
}
impl From<parser::ParseError> for Error {
    fn from(err: parser::ParseError) -> Self { Error::Parse(err) }
}
impl From<codegen::GenerateError> for Error {
    fn from(err: codegen::GenerateError) -> Self { Error::Generate(err) }
}
impl From<runtime::RuntimeError> for Error {
    fn from(err: runtime::RuntimeError) -> Self { Error::Runtime(err) }
}

/// Parses a `.flow` source file and returns the AST.
///
/// # Errors
///
/// Returns an `Error::Parse` if the source contains syntax errors.
pub fn parse_flow_file<P: AsRef<Path>>(path: P) -> Result<parser::ast::Ast, Error> {
    let src = fs::read_to_string(path)?;
    parser::parse(&src).map_err(Error::from)
}

/// Generates Rust source code from an AST.
///
/// The generated code is written to `output_path`.  Existing files are
/// overwritten.  Any errors during generation are returned as
/// `Error::Generate`.
pub fn generate_rust_code<P: AsRef<Path>>(
    ast: &parser::ast::Ast,
    output_path: P,
) -> Result<(), Error> {
    let code = codegen::generate(ast)?;
    fs::write(output_path, code).map_err(Error::from)
}

/// Compiles and runs a pipeline defined in `source_path`.  
///
/// The function first parses the source file, generates Rust code into
/// a temporary directory, compiles it using `rustc`, loads the resulting
/// library and executes the pipeline.  Errors at any stage are propagated.
pub fn run_pipeline<P: AsRef<Path>>(source_path: P) -> Result<(), Error> {
    let ast = parse_flow_file(&source_path)?;
    let tmp_dir = tempfile::tempdir()?;
    let lib_path = tmp_dir.path().join("pipeline.so");
    generate_rust_code(&ast, &lib_path)?;
    runtime::execute_library(&lib_path)
}

/// Reads a `.flow` file and returns the generated Rust source as a string.
///
/// This helper is useful for unit tests or for tooling that needs to
/// inspect the intermediate representation without touching the filesystem.
pub fn compile_flow_to_string(source: &str) -> Result<String, Error> {
    let ast = parser::parse(source)?;
    Ok(codegen::generate(&ast)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_parse_and_generate_basic() -> Result<(), Error> {
        // Load the example flow
        let example = include_str!("../examples/basic.flow");
        let ast = parser::parse(example)?;
        assert!(!ast.nodes.is_empty(), "AST should contain nodes");

        // Generate code into a temporary file
        let tmp_dir = tempfile::tempdir()?;
        let out_path = tmp_dir.path().join("basic.rs");
        generate_rust_code(&ast, &out_path)?;

        // Verify that the output file exists and is non‑empty
        assert!(out_path.exists(), "Output Rust file should exist");
        let content = fs::read_to_string(out_path)?;
        assert!(!content.is_empty(), "Generated code should not be empty");

        Ok(())
    }

    #[test]
    fn test_run_pipeline() -> Result<(), Error> {
        // Use the example flow file directly
        let path = PathBuf::from("../examples/basic.flow");
        run_pipeline(&path)?;
        Ok(())
    }

    #[test]
    fn test_compile_flow_to_string() -> Result<(), Error> {
        let source = "sensor: temp_sensor | act: fan";
        let code = compile_flow_to_string(source)?;
        assert!(code.contains("fn main"), "Generated code should contain a main function");
        Ok(())
    }
}