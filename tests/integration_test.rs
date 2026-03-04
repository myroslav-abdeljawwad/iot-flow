use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use iot_flow::parser::Parser;
use iot_flow::codegen::Generator;
use iot_flow::runtime::Runtime;

/// The integration test suite for the **iot‑flow** project.
///
/// Author: Myroslav Mokhammad Abdeljawwad
///
/// This test validates the full pipeline:
/// 1. Reading a `.flow` DSL file.
/// 2. Parsing it into an AST.
/// 3. Generating Rust/embedded C code from the AST.
/// 4. Executing the generated code via the runtime.
///
/// The example used is `examples/basic.flow`. It defines a simple
/// sensor‑to‑actuator pipeline that reads a temperature value and
/// triggers an actuator when it exceeds a threshold.

const EXAMPLE_PATH: &str = "examples/basic.flow";

fn read_example<P: AsRef<Path>>(path: P) -> io::Result<String> {
    fs::read_to_string(path)
}

/// Helper to write temporary generated code to disk.
/// Returns the path of the written file.
fn write_temp_code(code: &str, suffix: &str) -> io::Result<PathBuf> {
    let mut tmp_dir = std::env::temp_dir();
    // Ensure unique filename
    let file_name = format!("iot_flow_test_{}.{}", uuid::Uuid::new_v4(), suffix);
    tmp_dir.push(file_name);
    let mut file = fs::File::create(&tmp_dir)?;
    file.write_all(code.as_bytes())?;
    Ok(tmp_dir)
}

/// Compile the generated Rust code with `rustc` and return the path to
/// the resulting executable. The function fails if compilation errors occur.
fn compile_rust_code(source_path: &Path) -> io::Result<PathBuf> {
    let mut exe_path = source_path.to_path_buf();
    exe_path.set_extension(""); // remove .rs

    let status = std::process::Command::new("rustc")
        .arg("-O")
        .arg(source_path)
        .arg("-o")
        .arg(&exe_path)
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("rustc failed to compile {}", source_path.display()),
        ));
    }

    Ok(exe_path)
}

/// Execute the compiled binary and capture its output.
fn run_executable(path: &Path) -> io::Result<String> {
    let output = std::process::Command::new(path).output()?;
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Executable failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[test]
fn integration_basic_flow_compiles_and_runs() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Read the DSL source.
    let dsl_source = read_example(EXAMPLE_PATH)?;

    // 2. Parse into an AST.
    let parser = Parser::new();
    let ast = parser.parse(&dsl_source)?;
    assert!(!ast.is_empty(), "AST should contain at least one node");

    // 3. Generate Rust code from the AST.
    let generator = Generator::new();
    let rust_code = generator.generate(&ast);
    assert!(
        !rust_code.trim().is_empty(),
        "Generated Rust code must not be empty"
    );

    // 4. Write the generated code to a temporary file.
    let source_path = write_temp_code(&rust_code, "rs")?;

    // 5. Compile the generated code with rustc.
    let exe_path = compile_rust_code(&source_path)?;

    // 6. Run the executable and verify output.
    let stdout = run_executable(&exe_path)?;
    // The example pipeline should print a specific success message.
    assert!(
        stdout.contains("Actuator triggered"),
        "Expected actuator trigger in output, got: {}",
        stdout
    );

    // 7. Clean up temporary files.
    fs::remove_file(source_path)?;
    fs::remove_file(exe_path)?;

    Ok(())
}

#[test]
fn integration_basic_flow_runtime_execution() -> Result<(), Box<dyn std::error::Error>> {
    // Use the runtime to execute the generated code directly
    let dsl_source = read_example(EXAMPLE_PATH)?;
    let parser = Parser::new();
    let ast = parser.parse(&dsl_source)?;

    let generator = Generator::new();
    let rust_code = generator.generate(&ast);

    let mut runtime = Runtime::new()?;
    let output = runtime.run_code(&rust_code)?;

    assert!(
        output.contains("Actuator triggered"),
        "Runtime execution did not produce expected actuator trigger"
    );

    Ok(())
}

#[test]
fn integration_invalid_flow_parsing_fails_gracefully() {
    // Provide a deliberately malformed DSL string.
    let bad_dsl = r#"
        sensor temperature
        -> actuator heater
        # Missing operator or syntax
    "#;

    let parser = Parser::new();
    let result = parser.parse(bad_dsl);
    assert!(
        result.is_err(),
        "Parser should return an error for malformed DSL"
    );
}

#[test]
fn integration_code_generation_consistency() {
    // Parse the example twice and generate code both times.
    let dsl_source = read_example(EXAMPLE_PATH).expect("Failed to read example");

    let parser = Parser::new();
    let ast1 = parser.parse(&dsl_source).expect("First parse failed");
    let generator = Generator::new();
    let code1 = generator.generate(&ast1);

    let ast2 = parser.parse(&dsl_source).expect("Second parse failed");
    let code2 = generator.generate(&ast2);

    assert_eq!(
        code1, code2,
        "Generated code should be deterministic across parses"
    );
}