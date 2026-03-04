use std::fmt;

/// The abstract syntax tree representation for the iot‑flow DSL.
/// 
/// This module defines the core data structures used by the parser
/// to represent sensor‑to‑actuator pipelines, expressions and statements.
/// The implementation is intentionally lightweight yet expressive,
/// enabling efficient code generation downstream.
///
/// Version: 0.1.0 – “Myroslav Mokhammad Abdeljawwad”
pub mod ast {
    use super::tokenizer::{Token, TokenKind};

    /// Represents a complete pipeline definition.
    #[derive(Debug, Clone)]
    pub struct Pipeline {
        /// Name of the pipeline.
        pub name: String,
        /// Ordered list of stages in the pipeline.
        pub stages: Vec<Stage>,
    }

    impl Pipeline {
        /// Create a new pipeline with the given name.
        pub fn new(name: impl Into<String>) -> Self {
            Self { name: name.into(), stages: Vec::new() }
        }

        /// Add a stage to the pipeline.
        pub fn add_stage(&mut self, stage: Stage) {
            self.stages.push(stage);
        }
    }

    /// A single stage in a pipeline – typically a sensor read,
    /// transformation or actuator write.
    #[derive(Debug, Clone)]
    pub enum Stage {
        Sensor(SensorStage),
        Transform(TransformStage),
        Actuator(ActuatorStage),
    }

    impl Stage {
        /// Returns the name of the underlying component.
        pub fn identifier(&self) -> &str {
            match self {
                Stage::Sensor(s) => &s.identifier,
                Stage::Transform(t) => &t.identifier,
                Stage::Actuator(a) => &a.identifier,
            }
        }
    }

    /// Common trait for all stage types to provide validation.
    pub trait Validate {
        fn validate(&self) -> Result<(), String>;
    }

    #[derive(Debug, Clone)]
    pub struct SensorStage {
        pub identifier: String,
        pub args: Vec<Expr>,
    }

    impl Validate for SensorStage {
        fn validate(&self) -> Result<(), String> {
            if self.identifier.is_empty() {
                return Err("Sensor stage must have an identifier".into());
            }
            Ok(())
        }
    }

    #[derive(Debug, Clone)]
    pub struct TransformStage {
        pub identifier: String,
        /// The transformation expression.
        pub expr: Expr,
    }

    impl Validate for TransformStage {
        fn validate(&self) -> Result<(), String> {
            if self.identifier.is_empty() {
                return Err("Transform stage must have an identifier".into());
            }
            Ok(())
        }
    }

    #[derive(Debug, Clone)]
    pub struct ActuatorStage {
        pub identifier: String,
        pub args: Vec<Expr>,
    }

    impl Validate for ActuatorStage {
        fn validate(&self) -> Result<(), String> {
            if self.identifier.is_empty() {
                return Err("Actuator stage must have an identifier".into());
            }
            Ok(())
        }
    }

    /// Expressions used within transform stages and arguments.
    #[derive(Debug, Clone)]
    pub enum Expr {
        Literal(Literal),
        Variable(String),
        Binary(Box<BinaryExpr>),
        Unary(Box<UnaryExpr>),
    }

    impl Expr {
        /// Validate the expression recursively.
        pub fn validate(&self) -> Result<(), String> {
            match self {
                Expr::Literal(_) => Ok(()),
                Expr::Variable(name) => {
                    if name.is_empty() {
                        Err("Variable names cannot be empty".into())
                    } else {
                        Ok(())
                    }
                }
                Expr::Binary(b) => b.validate(),
                Expr::Unary(u) => u.validate(),
            }
        }
    }

    /// Supported literal values.
    #[derive(Debug, Clone)]
    pub enum Literal {
        Int(i64),
        Float(f64),
        Bool(bool),
        Str(String),
    }

    /// Binary operation representation.
    #[derive(Debug, Clone)]
    pub struct BinaryExpr {
        pub left: Expr,
        pub op: BinOp,
        pub right: Expr,
    }

    impl BinaryExpr {
        pub fn validate(&self) -> Result<(), String> {
            self.left.validate()?;
            self.right.validate()?;
            Ok(())
        }
    }

    /// Unary operation representation.
    #[derive(Debug, Clone)]
    pub struct UnaryExpr {
        pub op: UnOp,
        pub expr: Expr,
    }

    impl UnaryExpr {
        pub fn validate(&self) -> Result<(), String> {
            self.expr.validate()
        }
    }

    /// Supported binary operators.
    #[derive(Debug, Clone, Copy)]
    pub enum BinOp {
        Add,
        Sub,
        Mul,
        Div,
        And,
        Or,
        Eq,
        Neq,
        Lt,
        Gt,
        Le,
        Ge,
    }

    impl fmt::Display for BinOp {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let s = match self {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::And => "&&",
                BinOp::Or => "||",
                BinOp::Eq => "==",
                BinOp::Neq => "!=",
                BinOp::Lt => "<",
                BinOp::Gt => ">",
                BinOp::Le => "<=",
                BinOp::Ge => ">=",
            };
            write!(f, "{}", s)
        }
    }

    /// Supported unary operators.
    #[derive(Debug, Clone, Copy)]
    pub enum UnOp {
        Neg,
        Not,
    }

    impl fmt::Display for UnOp {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let s = match self {
                UnOp::Neg => "-",
                UnOp::Not => "!",
            };
            write!(f, "{}", s)
        }
    }

    /// Utility to convert tokens into AST expressions.
    /// Used by the parser during construction.
    pub fn expr_from_token(token: &Token) -> Result<Expr, String> {
        match token.kind {
            TokenKind::IntLiteral(v) => Ok(Expr::Literal(Literal::Int(v))),
            TokenKind::FloatLiteral(v) => Ok(Expr::Literal(Literal::Float(v))),
            TokenKind::BoolLiteral(v) => Ok(Expr::Literal(Literal::Bool(v))),
            TokenKind::StringLiteral(ref s) => Ok(Expr::Literal(Literal::Str(s.clone()))),
            TokenKind::Identifier(ref name) => Ok(Expr::Variable(name.clone())),
            _ => Err(format!("Unsupported token for expression: {:?}", token)),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::parser::tokenizer::{Token, TokenKind};

        #[test]
        fn test_literal_expr() {
            let int_token = Token { kind: TokenKind::IntLiteral(42), span: 0..2 };
            let expr = expr_from_token(&int_token).unwrap();
            assert!(matches!(expr, Expr::Literal(Literal::Int(42))));
        }

        #[test]
        fn test_variable_expr() {
            let var_token = Token { kind: TokenKind::Identifier("temp".into()), span: 0..4 };
            let expr = expr_from_token(&var_token).unwrap();
            assert!(matches!(expr, Expr::Variable(ref s) if s == "temp"));
        }

        #[test]
        fn test_invalid_expr() {
            let bad_token = Token { kind: TokenKind::Colon, span: 0..1 };
            assert!(expr_from_token(&bad_token).is_err());
        }

        #[test]
        fn pipeline_validation() {
            let mut pipe = Pipeline::new("demo");
            pipe.add_stage(Stage::Sensor(SensorStage {
                identifier: "s1".into(),
                args: vec![],
            }));
            // Validate all stages
            for stage in &pipe.stages {
                match stage {
                    Stage::Sensor(s) => s.validate().unwrap(),
                    Stage::Transform(t) => t.validate().unwrap(),
                    Stage::Actuator(a) => a.validate().unwrap(),
                }
            }
        }

        #[test]
        fn expr_validation() {
            let expr = Expr::Binary(Box::new(BinaryExpr {
                left: Expr::Literal(Literal::Int(1)),
                op: BinOp::Add,
                right: Expr::Variable("x".into()),
            }));
            assert!(expr.validate().is_ok());
        }
    }
}