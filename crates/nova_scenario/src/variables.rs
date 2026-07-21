use bevy::prelude::*;

use crate::prelude::*;

/// Glob-import surface: `use crate::variables::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{
        VariableConditionNode, VariableError, VariableExpressionNode, VariableFactorNode,
        VariableLiteral, VariableTermNode,
    };
}

/// Why evaluating a scenario-variable expression failed.
#[derive(Clone, Debug)]
pub enum VariableError {
    /// A referenced variable name is not set in the event world.
    UndefinedVariable(String),
    /// The operands' types are incompatible with the operation.
    TypeMismatch(String),
    /// A division expression had a zero divisor.
    DivisionByZero,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// A scenario variable's runtime value: the leaf of the variables DSL.
pub enum VariableLiteral {
    /// A string value.
    String(String),
    /// A numeric (f64) value.
    Number(f64),
    /// A boolean value.
    Boolean(bool),
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// A factor in the variables DSL: the atom of an expression (a parenthesized
/// subexpression, a literal, or a variable name).
pub enum VariableFactorNode {
    /// A parenthesized subexpression.
    Parens(Box<VariableExpressionNode>),
    /// A literal value.
    Literal(VariableLiteral),
    /// A reference to a variable by name, resolved against the event world.
    Name(String),
}

impl VariableFactorNode {
    /// Build a factor wrapping a literal value.
    pub fn new_literal(lit: VariableLiteral) -> Self {
        VariableFactorNode::Literal(lit)
    }

    /// Build a factor referencing a variable by name.
    pub fn new_name<S: Into<String>>(name: S) -> Self {
        VariableFactorNode::Name(name.into())
    }

    /// Build a factor wrapping a parenthesized subexpression.
    pub fn new_parens(expr: VariableExpressionNode) -> Self {
        VariableFactorNode::Parens(Box::new(expr))
    }

    /// Evaluate this factor against the event world's variable bindings.
    pub fn evaluate(&self, world: &NovaEventWorld) -> Result<VariableLiteral, VariableError> {
        match self {
            VariableFactorNode::Parens(expr) => expr.evaluate(world),
            VariableFactorNode::Literal(lit) => Ok(lit.clone()),
            VariableFactorNode::Name(name) => world
                .get_variable(name)
                .cloned()
                .ok_or_else(|| VariableError::UndefinedVariable(name.clone())),
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// A term in the variables DSL: factors joined by multiplication or division
/// (the precedence level above [`VariableExpressionNode`]'s add/subtract).
pub enum VariableTermNode {
    /// Multiply a factor by a term (numeric product, or boolean AND).
    Multiply(Box<VariableFactorNode>, Box<VariableTermNode>),
    /// Divide a factor by a term (numeric only; zero divisor is an error).
    Divide(Box<VariableFactorNode>, Box<VariableTermNode>),
    /// A bare factor with no multiplication or division.
    Factor(VariableFactorNode),
}

impl VariableTermNode {
    /// Build a multiplication term from a left factor and right term.
    pub fn new_multiply(left: VariableFactorNode, right: VariableTermNode) -> Self {
        VariableTermNode::Multiply(Box::new(left), Box::new(right))
    }

    /// Build a division term from a left factor and right term.
    pub fn new_divide(left: VariableFactorNode, right: VariableTermNode) -> Self {
        VariableTermNode::Divide(Box::new(left), Box::new(right))
    }

    /// Build a term that is a single factor.
    pub fn new_factor(factor: VariableFactorNode) -> Self {
        VariableTermNode::Factor(factor)
    }

    /// Evaluate this term against the event world's variable bindings.
    pub fn evaluate(&self, world: &NovaEventWorld) -> Result<VariableLiteral, VariableError> {
        match self {
            VariableTermNode::Multiply(left, right) => {
                let left_val = left.evaluate(world)?;
                let right_val = right.evaluate(world)?;
                match (left_val, right_val) {
                    (VariableLiteral::Number(l), VariableLiteral::Number(r)) => {
                        Ok(VariableLiteral::Number(l * r))
                    }
                    (VariableLiteral::Boolean(l), VariableLiteral::Boolean(r)) => {
                        Ok(VariableLiteral::Boolean(l && r))
                    }
                    (left_val, right_val) => Err(VariableError::TypeMismatch(
                        format!("evaluate: lhs and rhs must be numbers or booleans for multiplication, but got {:?} and {:?}", left_val, right_val)
                    )),
                }
            }
            VariableTermNode::Divide(left, right) => {
                let left_val = left.evaluate(world)?;
                let right_val = right.evaluate(world)?;
                match (left_val, right_val) {
                    (VariableLiteral::Number(l), VariableLiteral::Number(r)) => {
                        if r == 0.0 {
                            Err(VariableError::DivisionByZero)
                        } else {
                            Ok(VariableLiteral::Number(l / r))
                        }
                    }
                    (left_val, right_val) => Err(VariableError::TypeMismatch(format!(
                        "evaluate: lhs and rhs must be numbers for division, but got {:?} and {:?}",
                        left_val, right_val
                    ))),
                }
            }
            VariableTermNode::Factor(factor) => factor.evaluate(world),
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// An expression in the variables DSL: terms joined by addition or subtraction
/// (the top precedence level, the root of a value expression).
pub enum VariableExpressionNode {
    /// Add a term to an expression (numeric sum, boolean OR, or string concat).
    Add(Box<VariableTermNode>, Box<VariableExpressionNode>),
    /// Subtract an expression from a term (numeric only).
    Subtract(Box<VariableTermNode>, Box<VariableExpressionNode>),
    /// A bare term with no addition or subtraction.
    Term(VariableTermNode),
}

impl VariableExpressionNode {
    /// Build an addition expression from a left term and right expression.
    pub fn new_add(left: VariableTermNode, right: VariableExpressionNode) -> Self {
        VariableExpressionNode::Add(Box::new(left), Box::new(right))
    }

    /// Build a subtraction expression from a left term and right expression.
    pub fn new_subtract(left: VariableTermNode, right: VariableExpressionNode) -> Self {
        VariableExpressionNode::Subtract(Box::new(left), Box::new(right))
    }

    /// Build an expression that is a single term.
    pub fn new_term(term: VariableTermNode) -> Self {
        VariableExpressionNode::Term(term)
    }

    /// Evaluate this expression against the event world's variable bindings.
    pub fn evaluate(&self, world: &NovaEventWorld) -> Result<VariableLiteral, VariableError> {
        match self {
            VariableExpressionNode::Add(left, right) => {
                let left_val = left.evaluate(world)?;
                let right_val = right.evaluate(world)?;
                match (left_val, right_val) {
                    (VariableLiteral::Number(l), VariableLiteral::Number(r)) => {
                        Ok(VariableLiteral::Number(l + r))
                    }
                    (VariableLiteral::Boolean(l), VariableLiteral::Boolean(r)) => {
                        Ok(VariableLiteral::Boolean(l || r))
                    }
                    (VariableLiteral::String(l), VariableLiteral::String(r)) => {
                        Ok(VariableLiteral::String(l + &r))
                    }
                    (left_val, right_val) => Err(VariableError::TypeMismatch(
                        format!("evaluate: lhs and rhs must be numbers, booleans, or strings for addition, but got {:?} and {:?}", left_val, right_val)
                    )),
                }
            }
            VariableExpressionNode::Subtract(left, right) => {
                let left_val = left.evaluate(world)?;
                let right_val = right.evaluate(world)?;
                match (left_val, right_val) {
                    (VariableLiteral::Number(l), VariableLiteral::Number(r)) => {
                        Ok(VariableLiteral::Number(l - r))
                    }
                    (left_val, right_val) => Err(VariableError::TypeMismatch(
                        format!("evaluate: lhs and rhs must be numbers for subtraction, but got {:?} and {:?}", left_val, right_val)
                    )),
                }
            }
            VariableExpressionNode::Term(term) => term.evaluate(world),
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// A boolean condition in the variables DSL: two expressions compared, the
/// predicate a scenario event filter or gate evaluates.
pub enum VariableConditionNode {
    /// True when the left expression is numerically less than the right.
    LessThan(Box<VariableExpressionNode>, Box<VariableExpressionNode>),
    /// True when the left expression is numerically greater than the right.
    GreaterThan(Box<VariableExpressionNode>, Box<VariableExpressionNode>),
    /// True when both expressions are equal (numbers, booleans, or strings).
    Equal(Box<VariableExpressionNode>, Box<VariableExpressionNode>),
}

impl VariableConditionNode {
    /// Build a less-than comparison between two expressions.
    pub fn new_less_than(left: VariableExpressionNode, right: VariableExpressionNode) -> Self {
        VariableConditionNode::LessThan(Box::new(left), Box::new(right))
    }

    /// Build a greater-than comparison between two expressions.
    pub fn new_greater_than(left: VariableExpressionNode, right: VariableExpressionNode) -> Self {
        VariableConditionNode::GreaterThan(Box::new(left), Box::new(right))
    }

    /// Build an equality comparison between two expressions.
    pub fn new_equals(left: VariableExpressionNode, right: VariableExpressionNode) -> Self {
        VariableConditionNode::Equal(Box::new(left), Box::new(right))
    }

    /// Evaluate this condition against the event world's variable bindings.
    pub fn evaluate(&self, world: &NovaEventWorld) -> Result<bool, VariableError> {
        match self {
            VariableConditionNode::LessThan(left, right) => {
                let left_val = left.evaluate(world)?;
                let right_val = right.evaluate(world)?;
                match (left_val, right_val) {
                    (VariableLiteral::Number(l), VariableLiteral::Number(r)) => Ok(l < r),
                    (left_val, right_val) => Err(VariableError::TypeMismatch(
                        format!("evaluate: lhs and rhs must be numbers for less than comparison, but got {:?} and {:?}", left_val, right_val)
                    )),
                }
            }
            VariableConditionNode::GreaterThan(left, right) => {
                let left_val = left.evaluate(world)?;
                let right_val = right.evaluate(world)?;
                match (left_val, right_val) {
                    (VariableLiteral::Number(l), VariableLiteral::Number(r)) => Ok(l > r),
                    (left_val, right_val) => Err(VariableError::TypeMismatch(
                        format!("evaluate: lhs and rhs must be numbers for greater than comparison, but got {:?} and {:?}", left_val, right_val)
                    )),
                }
            }
            VariableConditionNode::Equal(left, right) => {
                let left_val = left.evaluate(world)?;
                let right_val = right.evaluate(world)?;
                match (left_val, right_val) {
                    (VariableLiteral::Number(l), VariableLiteral::Number(r)) => Ok(l == r),
                    (VariableLiteral::Boolean(l), VariableLiteral::Boolean(r)) => Ok(l == r),
                    (VariableLiteral::String(l), VariableLiteral::String(r)) => Ok(l == r),
                    (left_val, right_val) => Err(VariableError::TypeMismatch(
                        format!("evaluate: lhs and rhs must be of the same type for equality comparison, but got {:?} and {:?}", left_val, right_val)
                    )),
                }
            }
        }
    }
}
