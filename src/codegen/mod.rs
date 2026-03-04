use std::fmt;

pub mod generator;

/// The entry point for code generation.
///
/// This module exposes the `CodeGen` struct which orchestrates
/// conversion of a parsed AST into target language output.
/// It supports generating both Rust and embedded C code.
#[derive(Debug)]
pub struct CodeGen {
    /// Configuration options for the code generator.
    config: Config,
}

impl CodeGen {
    /// Creates a new `CodeGen` instance with default configuration.
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    /// Generates Rust source from the given AST.
    ///
    /// # Errors
    ///
    /// Returns an error if the generator encounters an unsupported construct or a rendering failure.
    pub fn generate_rust(&self, ast: &parser::ast::Pipeline) -> Result<String, CodeGenError> {
        generator::rust_generator::generate(ast)
    }

    /// Generates embedded C source from the given AST.
    ///
    /// # Errors
    ///
    /// Returns an error if the generator encounters an unsupported construct or a rendering failure.
    pub fn generate_c(&self, ast: &parser::ast::Pipeline) -> Result<String, CodeGenError> {
        generator::c_generator::generate(ast)
    }
}

/// Configuration options for the code generator.
///
/// These can be extended in the future to support custom formatting,
/// output directories, or feature flags.
#[derive(Debug)]
pub struct Config {
    /// Whether to enable verbose logging during generation.
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { verbose: false }
    }
}

/// Errors that can occur during code generation.
#[derive(Debug)]
pub enum CodeGenError {
    /// An error returned by the underlying generator implementation.
    Generation(String),
    /// The AST contains an unsupported construct.
    UnsupportedConstruct(String),
}

impl fmt::Display for CodeGenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generation(msg) => write!(f, "generation error: {}", msg),
            Self::UnsupportedConstruct(what) => write!(
                f,
                "unsupported construct in the pipeline: {}",
                what
            ),
        }
    }
}

impl std::error::Error for CodeGenError {}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::{ast::*, tokenizer};

    fn sample_pipeline() -> Pipeline {
        Pipeline {
            name: "sample".to_string(),
            steps: vec![
                Step::Sensor(SensorStep {
                    id: "temp_sensor".to_string(),
                    kind: SensorKind::Temperature,
                }),
                Step::Actuator(ActuatorStep {
                    id: "heater".to_string(),
                    kind: ActuatorKind::Relay,
                }),
                Step::Transform(TransformStep {
                    id: "threshold".to_string(),
                    function: TransformFunction::Threshold { value: 25.0 },
                }),
            ],
        }
    }

    #[test]
    fn test_generate_rust() {
        let gen = CodeGen::new();
        let pipeline = sample_pipeline();

        let output = gen.generate_rust(&pipeline).expect("Rust generation should succeed");
        assert!(output.contains("fn main"));
        assert!(output.contains("let temp_sensor"));
        assert!(output.contains("heater.activate"));
    }

    #[test]
    fn test_generate_c() {
        let gen = CodeGen::new();
        let pipeline = sample_pipeline();

        let output = gen.generate_c(&pipeline).expect("C generation should succeed");
        assert!(output.contains("#include <stdio.h>"));
        assert!(output.contains("int main(void)"));
        assert!(output.contains("temp_sensor_read()"));
        assert!(output.contains("heater_activate()"));
    }

    #[test]
    fn test_unsupported_construct_error() {
        // Create a pipeline with an unknown step to trigger the error.
        let mut bad_pipeline = sample_pipeline();
        bad_pipeline.steps.push(Step::Unknown);

        let gen = CodeGen::new();

        let result = gen.generate_rust(&bad_pipeline);
        assert!(matches!(
            result,
            Err(CodeGenError::UnsupportedConstruct(_))
        ));
    }
}