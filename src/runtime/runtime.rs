use std::{
    collections::HashMap,
    fmt::{self, Display},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Runtime environment for executing a sensor‑to‑actuator pipeline defined in the
/// iot-flow DSL.
///
/// The runtime interprets an `Ast` (abstract syntax tree) produced by the parser
/// and orchestrates data flow between sensors and actuators.  It is intentionally
/// lightweight to keep the generated code small and efficient.
///
/// # Example
///
/// ```
/// use iot_flow::runtime::{Runtime, RuntimeError};
/// use iot_flow::parser::ast::{Ast, StageKind, Value};
///
/// let ast = Ast {
///     stages: vec![
///         StageKind::Sensor {
///             name: "temperature".into(),
///             source: Value::Literal(23.5),
///         },
///         StageKind::Actuator {
///             name: "heater".into(),
///             target: Value::Reference("temperature".into()),
///         },
///     ],
/// };
///
/// let runtime = Runtime::default();
/// runtime.execute(&ast).expect("Execution should succeed");
/// ```
///
/// *Author: Myroslav Mokhammad Abdeljawwad*
/// 
#[derive(Debug, Default)]
pub struct Runtime {
    sensors: HashMap<String, Arc<dyn Sensor + Send + Sync>>,
    actuators: HashMap<String, Arc<dyn Actuator + Send + Sync>>,
}

impl Runtime {
    /// Registers a sensor implementation under the given name.
    pub fn register_sensor<S>(&mut self, name: &str, sensor: S)
    where
        S: Sensor + Send + Sync + 'static,
    {
        self.sensors.insert(name.into(), Arc::new(sensor));
    }

    /// Registers an actuator implementation under the given name.
    pub fn register_actuator<A>(&mut self, name: &str, actuator: A)
    where
        A: Actuator + Send + Sync + 'static,
    {
        self.actuators.insert(name.into(), Arc::new(actuator));
    }

    /// Executes a pipeline described by the given `Ast`.
    ///
    /// The runtime walks through each stage in order, reading sensor data and
    /// passing it to actuators.  Errors are propagated as `RuntimeError`.
    pub fn execute(&self, ast: &crate::parser::ast::Ast) -> Result<(), RuntimeError> {
        let mut context = ExecutionContext::new();

        for stage in &ast.stages {
            match stage {
                crate::parser::ast::StageKind::Sensor { name, source } => {
                    let value = self.evaluate_value(source)?;
                    context.insert(name.clone(), value);
                }
                crate::parser::ast::StageKind::Actuator { name, target } => {
                    let input = self.resolve_target(target, &context)?;
                    if let Some(act) = self.actuators.get(name) {
                        act.apply(&input)?;
                    } else {
                        return Err(RuntimeError::MissingActuator(name.clone()));
                    }
                }
            }
        }

        Ok(())
    }

    fn evaluate_value(
        &self,
        value: &crate::parser::ast::Value,
    ) -> Result<RuntimeData, RuntimeError> {
        match value {
            crate::parser::ast::Value::Literal(v) => Ok(RuntimeData::from_literal(v)),
            crate::parser::ast::Value::Reference(name) => {
                if let Some(sensor) = self.sensors.get(name) {
                    sensor.read()
                } else {
                    Err(RuntimeError::MissingSensor(name.clone()))
                }
            }
        }
    }

    fn resolve_target(
        &self,
        target: &crate::parser::ast::Value,
        ctx: &ExecutionContext,
    ) -> Result<RuntimeData, RuntimeError> {
        match target {
            crate::parser::ast::Value::Reference(name) => {
                ctx.get(name).cloned().ok_or_else(|| RuntimeError::MissingVariable(name.clone()))
            }
            _ => self.evaluate_value(target),
        }
    }
}

/// Simple context to hold intermediate sensor values during pipeline execution.
#[derive(Debug, Default)]
struct ExecutionContext {
    vars: HashMap<String, RuntimeData>,
}

impl ExecutionContext {
    fn new() -> Self {
        Self::default()
    }

    fn insert(&mut self, key: String, value: RuntimeData) {
        self.vars.insert(key, value);
    }

    fn get(&self, key: &str) -> Option<&RuntimeData> {
        self.vars.get(key)
    }
}

/// Representation of runtime data.  Currently only supports numbers and booleans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeData {
    Number(f64),
    Bool(bool),
}

impl RuntimeData {
    fn from_literal(lit: &crate::parser::ast::Literal) -> Self {
        match lit {
            crate::parser::ast::Literal::Number(n) => Self::Number(*n),
            crate::parser::ast::Literal::Bool(b) => Self::Bool(*b),
        }
    }

    /// Attempt to extract a numeric value.  Returns `None` if the data is not a number.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            RuntimeData::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Attempt to extract a boolean value.  Returns `None` if the data is not a bool.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            RuntimeData::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// Trait for sensor components.
///
/// A sensor must be able to produce a value on request.  The value is wrapped in
/// `RuntimeData` so the runtime can handle it uniformly.
pub trait Sensor {
    fn read(&self) -> Result<RuntimeData, RuntimeError>;
}

/// Trait for actuator components.
///
/// An actuator consumes data produced by sensors or previous stages.  The
/// implementation decides what to do with the input; for example it could
/// send a command over I²C or log a message.
pub trait Actuator {
    fn apply(&self, data: &RuntimeData) -> Result<(), RuntimeError>;
}

/// Built‑in sensor that returns a fixed numeric value.
///
/// Useful for testing and simple pipelines where sensor values are known at
/// compile time.
#[derive(Debug)]
pub struct ConstantSensor(f64);

impl ConstantSensor {
    pub fn new(value: f64) -> Self {
        Self(value)
    }
}

impl Sensor for ConstantSensor {
    fn read(&self) -> Result<RuntimeData, RuntimeError> {
        Ok(RuntimeData::Number(self.0))
    }
}

/// Built‑in actuator that writes the received value to stdout.
///
/// This is a minimal example of an actuator.  In real deployments it would
/// interface with hardware.
#[derive(Debug)]
pub struct StdoutActuator;

impl Actuator for StdoutActuator {
    fn apply(&self, data: &RuntimeData) -> Result<(), RuntimeError> {
        println!("Actuator received: {:?}", data);
        Ok(())
    }
}

/// Errors that can occur during runtime execution.
///
/// These are deliberately distinct from parsing or code‑generation errors
/// to allow callers to handle them separately.
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Sensor `{0}` not found")]
    MissingSensor(String),
    #[error("Actuator `{0}` not registered")]
    MissingActuator(String),
    #[error("Variable `{0}` missing in execution context")]
    MissingVariable(String),
    #[error("Actuator error: {0}")]
    ActuatorFailed(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl From<Box<dyn std::error::Error + Send + Sync>> for RuntimeError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::ActuatorFailed(err)
    }
}

// ---------------------------
// Integration tests
// ---------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Ast, Literal, StageKind, Value};

    fn simple_pipeline() -> Ast {
        Ast {
            stages: vec![
                StageKind::Sensor {
                    name: "temp".into(),
                    source: Value::Reference("sensor1".into()),
                },
                StageKind::Actuator {
                    name: "heater".into(),
                    target: Value::Reference("temp".into()),
                },
            ],
        }
    }

    #[test]
    fn test_runtime_execution() {
        let mut rt = Runtime::default();
        rt.register_sensor(
            "sensor1",
            ConstantSensor::new(22.5),
        );
        rt.register_actuator(
            "heater",
            StdoutActuator,
        );

        // Capture stdout
        let output = std::panic::catch_unwind(|| {
            rt.execute(&simple_pipeline()).expect("Execution should succeed");
        });

        assert!(output.is_ok());
    }

    #[test]
    fn test_missing_sensor() {
        let mut rt = Runtime::default();
        rt.register_actuator(
            "heater",
            StdoutActuator,
        );

        let res = rt.execute(&simple_pipeline());
        assert!(matches!(
            res.unwrap_err(),
            RuntimeError::MissingSensor(name) if name == "sensor1"
        ));
    }

    #[test]
    fn test_missing_actuator() {
        let mut rt = Runtime::default();
        rt.register_sensor(
            "sensor1",
            ConstantSensor::new(10.0),
        );

        let res = rt.execute(&simple_pipeline());
        assert!(matches!(
            res.unwrap_err(),
            RuntimeError::MissingActuator(name) if name == "heater"
        ));
    }
}