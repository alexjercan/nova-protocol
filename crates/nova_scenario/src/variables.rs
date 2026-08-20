//! The scenario variable expression grammar and its evaluator.
//!
//! A hand-written precedence chain - expression over term over factor - so
//! authored arithmetic and comparisons evaluate against live world state
//! without pulling in a parser dependency.
//!
//! Touch this module when adding an operator or literal kind an author can
//! write.

use bevy::prelude::*;

use crate::prelude::*;

/// The scenario variable expression nodes, `VariableLiteral`, `VariableError` and `EQUAL_EPSILON`.
pub mod prelude {
    pub use super::{
        collect_condition_queries, collect_expression_queries, VariableConditionNode,
        VariableError, VariableExpressionNode, VariableFactorNode, VariableLiteral,
        VariableTermNode, EQUAL_EPSILON,
    };
}

/// Why evaluating a scenario-variable expression failed.
#[derive(Clone, Debug)]
pub enum VariableError {
    /// A referenced variable name is not set in the event world.
    UndefinedVariable(String),
    /// A typed query has no value in the current world snapshot.
    UnavailableQuery(QueryConfig),
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
    /// A typed read-only query against the current world snapshot.
    Query(QueryConfig),
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

    /// Build a factor wrapping a typed read-only query.
    pub fn new_query(query: QueryConfig) -> Self {
        VariableFactorNode::Query(query)
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
            VariableFactorNode::Query(query) => world
                .query_value(query)
                .cloned()
                .ok_or_else(|| VariableError::UnavailableQuery(query.clone())),
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

/// How close two numbers must be for the DSL's `Equal` to call them equal.
///
/// `Equal` was exact float equality, so an author writing
/// `Equal(hull_fraction, 0.5)` saw the condition essentially never fire, with
/// no error and no warning. The DSL's numbers are fractions, seconds and
/// counts - all small - so an absolute tolerance is the right shape.
pub const EQUAL_EPSILON: f64 = 1e-6;

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
                    (VariableLiteral::Number(l), VariableLiteral::Number(r)) => {
                        Ok((l - r).abs() <= EQUAL_EPSILON)
                    }
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

/// Every typed query a comparison reads, in source order.
pub fn collect_condition_queries<'a>(
    node: &'a VariableConditionNode,
    out: &mut Vec<&'a QueryConfig>,
) {
    match node {
        VariableConditionNode::LessThan(left, right)
        | VariableConditionNode::GreaterThan(left, right)
        | VariableConditionNode::Equal(left, right) => {
            collect_expression_queries(left, out);
            collect_expression_queries(right, out);
        }
    }
}

/// Every typed query an expression reads, in source order.
///
/// Two callers walk the same tree and must not drift: the lint checks each
/// query's target against what the scenario can spawn, and the loader decides
/// from the same list whether the per-frame entity sampler has a reader at all.
pub fn collect_expression_queries<'a>(
    node: &'a VariableExpressionNode,
    out: &mut Vec<&'a QueryConfig>,
) {
    match node {
        VariableExpressionNode::Add(term, rest) | VariableExpressionNode::Subtract(term, rest) => {
            collect_term_queries(term, out);
            collect_expression_queries(rest, out);
        }
        VariableExpressionNode::Term(term) => collect_term_queries(term, out),
    }
}

fn collect_term_queries<'a>(node: &'a VariableTermNode, out: &mut Vec<&'a QueryConfig>) {
    match node {
        VariableTermNode::Multiply(factor, rest) | VariableTermNode::Divide(factor, rest) => {
            collect_factor_queries(factor, out);
            collect_term_queries(rest, out);
        }
        VariableTermNode::Factor(factor) => collect_factor_queries(factor, out),
    }
}

fn collect_factor_queries<'a>(node: &'a VariableFactorNode, out: &mut Vec<&'a QueryConfig>) {
    match node {
        VariableFactorNode::Parens(inner) => collect_expression_queries(inner, out),
        VariableFactorNode::Query(query) => out.push(query),
        VariableFactorNode::Literal(_) | VariableFactorNode::Name(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Term(Factor(Parens(` ... `)))` repeated `levels` times around a literal.
    fn nested_expr_ron(levels: usize) -> String {
        let mut out = String::new();
        for _ in 0..levels {
            out.push_str("Term(Factor(Parens(");
        }
        out.push_str("Term(Factor(Literal(Number(1.0))))");
        for _ in 0..levels {
            out.push_str(")))");
        }
        out
    }

    /// F09 - RULED NOT A DEFECT, pinned so it stays that way.
    ///
    /// The variables DSL is `Box`-recursive with no depth field of its own, so
    /// authored content looks able to overflow the stack inside the decode, on
    /// the asset-loader task, during boot. It cannot: `ron::de::from_bytes`
    /// (the call every production decode site uses -
    /// `nova_modding/src/lib.rs:194,234,340`) runs under `Options::default()`,
    /// whose `recursion_limit` is `Some(128)`. Deep nesting is a parse ERROR.
    ///
    /// This test fails if someone reaches for `without_recursion_limit()` or
    /// swaps in a format with no bound - which is the only way the overflow
    /// becomes real. Bounding the grammar itself would be dead machinery.
    #[test]
    fn a_decode_deeper_than_rons_recursion_limit_is_refused_not_walked() {
        let deep = nested_expr_ron(4096);
        assert!(
            ron::de::from_bytes::<VariableExpressionNode>(deep.as_bytes()).is_err(),
            "an over-deep authored expression is refused at decode"
        );
        assert!(
            ron::de::from_bytes::<VariableExpressionNode>(nested_expr_ron(4).as_bytes()).is_ok(),
            "nesting an author would actually write still decodes"
        );
    }

    #[test]
    fn query_factor_reads_a_typed_snapshot_and_can_be_captured() {
        let query = QueryConfig::Scenario(ScenarioQuery {
            property: ScenarioProperty::Elapsed,
        });
        let factor = VariableFactorNode::new_query(query.clone());
        let mut world = NovaEventWorld::default();
        assert!(matches!(
            factor.evaluate(&world),
            Err(VariableError::UnavailableQuery(unavailable)) if unavailable == query
        ));

        world.advance_scenario_elapsed(2.5);
        assert_eq!(
            factor.evaluate(&world).expect("query is sampled"),
            VariableLiteral::Number(2.5)
        );
    }

    #[test]
    fn typed_queries_round_trip_through_ron() {
        let factor = VariableFactorNode::new_query(QueryConfig::Entity(EntityQuery {
            filter: EntityQueryFilter {
                id: "courier".to_string(),
            },
            property: EntityProperty::Speed,
        }));
        let ron = ron::to_string(&factor).expect("serialize query factor");
        let back: VariableFactorNode = ron::from_str(&ron).expect("parse query factor");
        assert!(matches!(
            back,
            VariableFactorNode::Query(QueryConfig::Entity(_))
        ));
    }

    /// F61: `Equal` was exact float equality, so a condition an author wrote
    /// against a computed fraction essentially never fired.
    #[test]
    fn equal_compares_numbers_within_an_epsilon() {
        let world = NovaEventWorld::default();
        let lit = |n: f64| {
            VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Number(n)),
            ))
        };

        // 0.1 + 0.2 != 0.3 exactly. This is the shape authored content hits.
        let sum = VariableExpressionNode::new_add(
            VariableTermNode::new_factor(VariableFactorNode::new_literal(VariableLiteral::Number(
                0.1,
            ))),
            lit(0.2),
        );
        assert!(VariableConditionNode::new_equals(sum, lit(0.3))
            .evaluate(&world)
            .expect("evaluates"));

        // Still a comparison, not "always true".
        assert!(!VariableConditionNode::new_equals(lit(0.5), lit(0.6))
            .evaluate(&world)
            .expect("evaluates"));
    }
}
