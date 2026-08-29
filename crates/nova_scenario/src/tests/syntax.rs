//! Round-trip tests for the expression text syntax.

use crate::prelude::*;

/// Text -> tree -> text, which is the half a panel depends on: what a person
/// typed is what the panel shows back.
fn expression_round_trips(text: &str) {
    let parsed: VariableExpressionNode = text.parse().expect(text);
    assert_eq!(parsed.to_string(), text, "expression '{text}'");
}

fn condition_round_trips(text: &str) {
    let parsed: VariableConditionNode = text.parse().expect(text);
    assert_eq!(parsed.to_string(), text, "condition '{text}'");
}

#[test]
fn every_shape_the_grammar_has_round_trips_through_its_text() {
    for text in [
        "score",
        "12",
        "-3.5",
        "true",
        "false",
        "\"a line\"",
        "scenario.elapsed",
        "entity(\"player_spaceship\").speed",
        "a + b",
        "a - b",
        "a * b",
        "a / b",
        "(a + b) * c",
        "a + b * c",
        "a - b - c",
        "scenario.elapsed / 60",
        "entity(\"player_spaceship\").speed * 2 + 1",
    ] {
        expression_round_trips(text);
    }
    for text in [
        "picket_warden_awake == false",
        "scenario.elapsed > 90",
        "score < 10",
        "entity(\"player_spaceship\").speed > 50",
        "kills == 3",
    ] {
        condition_round_trips(text);
    }
}

#[test]
fn a_tree_built_in_code_renders_as_the_text_that_parses_back_to_it() {
    let built = VariableConditionNode::new_equals(
        VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_name("picket_warden_awake"),
        )),
        VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_literal(VariableLiteral::Boolean(false)),
        )),
    );
    let text = built.to_string();
    assert_eq!(text, "picket_warden_awake == false");

    let parsed: VariableConditionNode = text.parse().expect("parses");
    assert_eq!(parsed.to_string(), text);
}

#[test]
fn a_string_holding_a_quote_survives_the_round_trip() {
    let built = VariableExpressionNode::new_term(VariableTermNode::new_factor(
        VariableFactorNode::new_literal(VariableLiteral::String("say \"go\"".to_string())),
    ));
    let text = built.to_string();
    let parsed: VariableExpressionNode = text.parse().expect("parses");
    assert_eq!(parsed.to_string(), text);
}

#[test]
fn a_minus_is_an_operator_between_values_and_a_sign_before_one() {
    let subtraction: VariableExpressionNode = "a - 3".parse().expect("parses");
    assert!(matches!(
        subtraction,
        VariableExpressionNode::Subtract(_, _)
    ));

    let negative: VariableExpressionNode = "-3".parse().expect("parses");
    assert_eq!(negative.to_string(), "-3");
}

#[test]
fn text_the_grammar_cannot_hold_is_refused_with_a_reason() {
    for text in [
        "a +",
        "(a",
        "a ** b",
        "scenario.heading",
        "entity(player).speed",
        "a & b",
        "\"unterminated",
        "entity(\"x\")",
    ] {
        let parsed = text.parse::<VariableExpressionNode>();
        assert!(parsed.is_err(), "'{text}' should not parse");
        assert!(
            !parsed.unwrap_err().message().is_empty(),
            "'{text}' needs a reason"
        );
    }
}

#[test]
fn a_condition_needs_a_comparison() {
    let error = "a + b".parse::<VariableConditionNode>().unwrap_err();
    assert!(error.message().contains("compares"));
}

#[test]
fn trailing_text_is_an_error_rather_than_silently_dropped() {
    let error = "a b".parse::<VariableExpressionNode>().unwrap_err();
    assert!(error.message().contains("unexpected"), "{error}");
}
