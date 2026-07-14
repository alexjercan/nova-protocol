use bevy::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        VariableConditionNode, VariableError, VariableExpressionNode, VariableFactorNode,
        VariableLiteral, VariableTermNode,
    };
}

#[derive(Clone, Debug)]
pub enum VariableError {
    UndefinedVariable(String),
    TypeMismatch(String),
    DivisionByZero,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VariableLiteral {
    String(String),
    Number(f64),
    Boolean(bool),
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VariableFactorNode {
    Parens(Box<VariableExpressionNode>),
    Literal(VariableLiteral),
    Name(String),
}

impl VariableFactorNode {
    pub fn new_literal(lit: VariableLiteral) -> Self {
        VariableFactorNode::Literal(lit)
    }

    pub fn new_name<S: Into<String>>(name: S) -> Self {
        VariableFactorNode::Name(name.into())
    }

    pub fn new_parens(expr: VariableExpressionNode) -> Self {
        VariableFactorNode::Parens(Box::new(expr))
    }

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
pub enum VariableTermNode {
    Multiply(Box<VariableFactorNode>, Box<VariableTermNode>),
    Divide(Box<VariableFactorNode>, Box<VariableTermNode>),
    Factor(VariableFactorNode),
}

impl VariableTermNode {
    pub fn new_multiply(left: VariableFactorNode, right: VariableTermNode) -> Self {
        VariableTermNode::Multiply(Box::new(left), Box::new(right))
    }

    pub fn new_divide(left: VariableFactorNode, right: VariableTermNode) -> Self {
        VariableTermNode::Divide(Box::new(left), Box::new(right))
    }

    pub fn new_factor(factor: VariableFactorNode) -> Self {
        VariableTermNode::Factor(factor)
    }

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
pub enum VariableExpressionNode {
    Add(Box<VariableTermNode>, Box<VariableExpressionNode>),
    Subtract(Box<VariableTermNode>, Box<VariableExpressionNode>),
    Term(VariableTermNode),
}

impl VariableExpressionNode {
    pub fn new_add(left: VariableTermNode, right: VariableExpressionNode) -> Self {
        VariableExpressionNode::Add(Box::new(left), Box::new(right))
    }

    pub fn new_subtract(left: VariableTermNode, right: VariableExpressionNode) -> Self {
        VariableExpressionNode::Subtract(Box::new(left), Box::new(right))
    }

    pub fn new_term(term: VariableTermNode) -> Self {
        VariableExpressionNode::Term(term)
    }

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
pub enum VariableConditionNode {
    LessThan(Box<VariableExpressionNode>, Box<VariableExpressionNode>),
    GreaterThan(Box<VariableExpressionNode>, Box<VariableExpressionNode>),
    Equal(Box<VariableExpressionNode>, Box<VariableExpressionNode>),
}

impl VariableConditionNode {
    pub fn new_less_than(left: VariableExpressionNode, right: VariableExpressionNode) -> Self {
        VariableConditionNode::LessThan(Box::new(left), Box::new(right))
    }

    pub fn new_greater_than(left: VariableExpressionNode, right: VariableExpressionNode) -> Self {
        VariableConditionNode::GreaterThan(Box::new(left), Box::new(right))
    }

    pub fn new_equals(left: VariableExpressionNode, right: VariableExpressionNode) -> Self {
        VariableConditionNode::Equal(Box::new(left), Box::new(right))
    }

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
