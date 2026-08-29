//! The TEXT form of the variables grammar: what a person types, and what a
//! panel shows.
//!
//! The authored form of an expression is RON - a nest of `Add(Term(Factor(...`
//! that reads as the tree it is. That is fine in a file and unreadable in a
//! 300px inspector row, where reflection would draw one row per node and a
//! two-term comparison would fill the panel. So the grammar gets a surface
//! syntax: `picket_warden_awake == false`, `scenario.elapsed > 90`,
//! `entity("player_spaceship").speed * 2`.
//!
//! ROUND-TRIP IS THE CONTRACT, in both directions. Parsing a rendering gives
//! back the same tree, and rendering a parse gives back the same text; the
//! tests below hold both. That is what lets a panel own a handler's condition
//! without a save quietly rewriting it.
//!
//! The syntax is the grammar and nothing more - no operator this crate cannot
//! evaluate, and no precedence the tree cannot express. `a - b - c` parses
//! RIGHT-associatively because `Subtract(Term, Expression)` nests that way;
//! writing it left-associatively would be a syntax that means something the
//! authored form cannot hold.

use core::{fmt, str::FromStr};

use crate::prelude::*;

/// Glob-import surface: `use nova_scenario::syntax::prelude::*` brings the
/// parse error into scope. The `Display` and `FromStr` impls need no import.
pub mod prelude {
    pub use super::SyntaxError;
}

/// Why a piece of expression text is not an expression.
///
/// One flat kind with a message rather than a variant per failure: every
/// consumer shows it to whoever typed the text, and none of them branches on
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxError(String);

impl SyntaxError {
    /// The message, ready to put under a field.
    pub fn message(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl core::error::Error for SyntaxError {}

impl fmt::Display for VariableLiteral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Escaped, so a line holding a quote survives a round trip.
            VariableLiteral::String(text) => write!(f, "\"{}\"", text.replace('"', "\\\"")),
            VariableLiteral::Number(value) => write!(f, "{value}"),
            VariableLiteral::Boolean(value) => write!(f, "{value}"),
        }
    }
}

impl fmt::Display for QueryConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryConfig::Scenario(query) => match query.property {
                ScenarioProperty::Elapsed => f.write_str("scenario.elapsed"),
            },
            QueryConfig::Entity(query) => {
                write!(f, "entity(\"{}\").", query.filter.id)?;
                match query.property {
                    EntityProperty::Speed => f.write_str("speed"),
                }
            }
        }
    }
}

impl fmt::Display for VariableFactorNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariableFactorNode::Parens(inner) => write!(f, "({inner})"),
            VariableFactorNode::Literal(literal) => write!(f, "{literal}"),
            VariableFactorNode::Name(name) => f.write_str(name),
            VariableFactorNode::Query(query) => write!(f, "{query}"),
        }
    }
}

impl fmt::Display for VariableTermNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariableTermNode::Multiply(left, right) => write!(f, "{left} * {right}"),
            VariableTermNode::Divide(left, right) => write!(f, "{left} / {right}"),
            VariableTermNode::Factor(factor) => write!(f, "{factor}"),
        }
    }
}

impl fmt::Display for VariableExpressionNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariableExpressionNode::Add(left, right) => write!(f, "{left} + {right}"),
            VariableExpressionNode::Subtract(left, right) => write!(f, "{left} - {right}"),
            VariableExpressionNode::Term(term) => write!(f, "{term}"),
        }
    }
}

impl fmt::Display for VariableConditionNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariableConditionNode::LessThan(left, right) => write!(f, "{left} < {right}"),
            VariableConditionNode::GreaterThan(left, right) => write!(f, "{left} > {right}"),
            VariableConditionNode::Equal(left, right) => write!(f, "{left} == {right}"),
        }
    }
}

impl FromStr for VariableExpressionNode {
    type Err = SyntaxError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut parser = Parser::new(text)?;
        let expression = parser.expression()?;
        parser.finish()?;
        Ok(expression)
    }
}

impl FromStr for VariableConditionNode {
    type Err = SyntaxError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut parser = Parser::new(text)?;
        let condition = parser.condition()?;
        parser.finish()?;
        Ok(condition)
    }
}

/// One lexed piece of expression text.
#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Text(String),
    Boolean(bool),
    Name(String),
    Symbol(&'static str),
}

/// Every symbol the grammar spells, longest first so `==` is never read as two
/// `=`.
const SYMBOLS: [&str; 10] = ["==", "+", "-", "*", "/", "(", ")", "<", ">", "."];

/// Split `text` into tokens.
fn lex(text: &str) -> Result<Vec<Token>, SyntaxError> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        if c.is_whitespace() {
            at += 1;
            continue;
        }
        if c == '"' {
            let mut value = String::new();
            at += 1;
            loop {
                let Some(&c) = chars.get(at) else {
                    return Err(SyntaxError("unterminated string".to_string()));
                };
                at += 1;
                match c {
                    '"' => break,
                    '\\' => match chars.get(at) {
                        Some(&escaped) => {
                            value.push(escaped);
                            at += 1;
                        }
                        None => return Err(SyntaxError("unterminated string".to_string())),
                    },
                    _ => value.push(c),
                }
            }
            tokens.push(Token::Text(value));
            continue;
        }
        if c.is_ascii_digit() {
            let start = at;
            while chars
                .get(at)
                .is_some_and(|c| c.is_ascii_digit() || *c == '.')
            {
                at += 1;
            }
            let literal: String = chars[start..at].iter().collect();
            let value = literal
                .parse::<f64>()
                .map_err(|_| SyntaxError(format!("'{literal}' is not a number")))?;
            tokens.push(Token::Number(value));
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = at;
            while chars
                .get(at)
                .is_some_and(|c| c.is_alphanumeric() || *c == '_')
            {
                at += 1;
            }
            let word: String = chars[start..at].iter().collect();
            tokens.push(match word.as_str() {
                "true" => Token::Boolean(true),
                "false" => Token::Boolean(false),
                _ => Token::Name(word),
            });
            continue;
        }
        let rest: String = chars[at..].iter().collect();
        let Some(symbol) = SYMBOLS.iter().find(|symbol| rest.starts_with(**symbol)) else {
            return Err(SyntaxError(format!("'{c}' is not part of an expression")));
        };
        tokens.push(Token::Symbol(symbol));
        at += symbol.chars().count();
    }
    Ok(tokens)
}

/// A cursor over lexed tokens, one method per grammar level.
struct Parser {
    tokens: Vec<Token>,
    at: usize,
}

impl Parser {
    fn new(text: &str) -> Result<Self, SyntaxError> {
        Ok(Self {
            tokens: lex(text)?,
            at: 0,
        })
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    /// Consume `symbol` if it is next.
    fn eat(&mut self, symbol: &str) -> bool {
        if matches!(self.peek(), Some(Token::Symbol(next)) if *next == symbol) {
            self.at += 1;
            return true;
        }
        false
    }

    /// Error unless every token was consumed.
    fn finish(&self) -> Result<(), SyntaxError> {
        match self.peek() {
            None => Ok(()),
            Some(token) => Err(SyntaxError(format!("unexpected {}", describe(token)))),
        }
    }

    fn condition(&mut self) -> Result<VariableConditionNode, SyntaxError> {
        let left = self.expression()?;
        if self.eat("==") {
            return Ok(VariableConditionNode::new_equals(left, self.expression()?));
        }
        if self.eat("<") {
            return Ok(VariableConditionNode::new_less_than(
                left,
                self.expression()?,
            ));
        }
        if self.eat(">") {
            return Ok(VariableConditionNode::new_greater_than(
                left,
                self.expression()?,
            ));
        }
        Err(SyntaxError(
            "a condition compares two expressions with <, > or ==".to_string(),
        ))
    }

    fn expression(&mut self) -> Result<VariableExpressionNode, SyntaxError> {
        let left = self.term()?;
        if self.eat("+") {
            return Ok(VariableExpressionNode::new_add(left, self.expression()?));
        }
        if self.eat("-") {
            return Ok(VariableExpressionNode::new_subtract(
                left,
                self.expression()?,
            ));
        }
        Ok(VariableExpressionNode::new_term(left))
    }

    fn term(&mut self) -> Result<VariableTermNode, SyntaxError> {
        let left = self.factor()?;
        if self.eat("*") {
            return Ok(VariableTermNode::new_multiply(left, self.term()?));
        }
        if self.eat("/") {
            return Ok(VariableTermNode::new_divide(left, self.term()?));
        }
        Ok(VariableTermNode::new_factor(left))
    }

    fn factor(&mut self) -> Result<VariableFactorNode, SyntaxError> {
        // A LEADING minus is read here and only here. The grammar has no unary
        // operator, so `-3` is the literal -3 - and it is only a literal in a
        // position where a factor may start, which is what keeps the `-` of
        // `a - 3` an operator.
        if matches!(self.peek(), Some(Token::Symbol("-")))
            && matches!(self.tokens.get(self.at + 1), Some(Token::Number(_)))
        {
            self.at += 1;
            let Some(Token::Number(value)) = self.peek().cloned() else {
                unreachable!("the token after the minus was just matched as a number");
            };
            self.at += 1;
            return Ok(VariableFactorNode::new_literal(VariableLiteral::Number(
                -value,
            )));
        }
        if self.eat("(") {
            let inner = self.expression()?;
            if !self.eat(")") {
                return Err(SyntaxError("missing ')'".to_string()));
            }
            return Ok(VariableFactorNode::new_parens(inner));
        }
        let Some(token) = self.peek().cloned() else {
            return Err(SyntaxError("the expression ends early".to_string()));
        };
        self.at += 1;
        match token {
            Token::Number(value) => Ok(VariableFactorNode::new_literal(VariableLiteral::Number(
                value,
            ))),
            Token::Text(value) => Ok(VariableFactorNode::new_literal(VariableLiteral::String(
                value,
            ))),
            Token::Boolean(value) => Ok(VariableFactorNode::new_literal(VariableLiteral::Boolean(
                value,
            ))),
            Token::Name(name) => self.named(name),
            Token::Symbol(symbol) => Err(SyntaxError(format!("'{symbol}' cannot start a value"))),
        }
    }

    /// A word: a query the grammar knows, or a variable name.
    ///
    /// The two query spellings are the two the grammar has. Anything else is a
    /// variable, which is a bare word - so a dotted word is always a misspelt
    /// query and says so rather than becoming a variable nothing ever sets.
    fn named(&mut self, name: String) -> Result<VariableFactorNode, SyntaxError> {
        match name.as_str() {
            "scenario" => {
                self.property("scenario")?;
                match self.property_name("scenario")?.as_str() {
                    "elapsed" => Ok(VariableFactorNode::new_query(QueryConfig::Scenario(
                        ScenarioQuery {
                            property: ScenarioProperty::Elapsed,
                        },
                    ))),
                    other => Err(SyntaxError(format!("'{other}' is not a scenario property"))),
                }
            }
            "entity" => {
                if !self.eat("(") {
                    return Err(SyntaxError("entity needs an id: entity(\"x\")".to_string()));
                }
                let Some(Token::Text(id)) = self.peek().cloned() else {
                    return Err(SyntaxError(
                        "an entity id is a quoted string: entity(\"x\")".to_string(),
                    ));
                };
                self.at += 1;
                if !self.eat(")") {
                    return Err(SyntaxError("missing ')' after the entity id".to_string()));
                }
                self.property("entity")?;
                match self.property_name("entity")?.as_str() {
                    "speed" => Ok(VariableFactorNode::new_query(QueryConfig::Entity(
                        EntityQuery {
                            filter: EntityQueryFilter { id },
                            property: EntityProperty::Speed,
                        },
                    ))),
                    other => Err(SyntaxError(format!("'{other}' is not an entity property"))),
                }
            }
            _ => Ok(VariableFactorNode::new_name(name)),
        }
    }

    /// The dot of a query's property access.
    fn property(&mut self, subject: &str) -> Result<(), SyntaxError> {
        self.eat(".")
            .then_some(())
            .ok_or_else(|| SyntaxError(format!("{subject} reads a property after a '.'")))
    }

    /// The word after that dot.
    fn property_name(&mut self, subject: &str) -> Result<String, SyntaxError> {
        let Some(Token::Name(property)) = self.peek().cloned() else {
            return Err(SyntaxError(format!("{subject} needs a property name")));
        };
        self.at += 1;
        Ok(property)
    }
}

/// What a token is, for an error message.
fn describe(token: &Token) -> String {
    match token {
        Token::Number(value) => format!("number {value}"),
        Token::Text(value) => format!("string \"{value}\""),
        Token::Boolean(value) => format!("{value}"),
        Token::Name(name) => format!("'{name}'"),
        Token::Symbol(symbol) => format!("'{symbol}'"),
    }
}
