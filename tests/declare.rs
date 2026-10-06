//! Tests for the `declare` macros, against the existing `Dfa` composition.

use std::ops::RangeInclusive;

use range_map_regex::{
    declare::{dfa, float_literal, fsm},
    dfa::Dfa,
    fsm::StateMachine,
};

/// The float literal from `examples/float_literals.rs`.
fn original_float_literal() -> Dfa {
    let digit = Dfa::from_char_range('0'..='9');
    let digits = digit
        .plus()
        .concat(&Dfa::from_char('_').concat(&digit.plus()).star());
    let exponent = Dfa::from_char('e')
        .union(&Dfa::from_char('E'))
        .concat(&Dfa::from_char('+').union(&Dfa::from_char('-')).optional())
        .concat(&digits);
    let suffix = Dfa::string("f32").union(&Dfa::string("f64"));
    let optional_suffix = Dfa::from_char('_').optional().concat(&suffix).optional();
    digits
        .concat(&Dfa::from_char('.'))
        .concat(&digits.optional())
        .concat(&exponent.optional())
        .concat(&optional_suffix)
        .union(&digits.concat(&exponent).concat(&optional_suffix))
        .union(&digits.concat(&suffix))
}

float_literal! {
    RustFloat { separator: '_', suffixes: ["f32", "f64"] }
}

dfa! {
    FloatRules {
        start: Float,
        Digits { is: seq([many1(chars('0'..='9')), many(seq([lit("_"), many1(chars('0'..='9'))]))]) },
        Exponent { is: seq([one_of("eE"), opt(one_of("+-")), digits()]) },
        Suffix { is: alt([lit("f32"), lit("f64")]) },
        Float {
            is: alt([
                seq([digits(), lit("."), opt(digits()), opt(exponent()), opt(seq([opt(lit("_")), suffix()]))]),
                seq([digits(), exponent(), opt(seq([opt(lit("_")), suffix()]))]),
                seq([digits(), suffix()]),
            ]),
        },
    }
}

#[test]
fn float_declarations_match_the_original() {
    let original = original_float_literal();
    assert!(RustFloat::dfa().is_equivalent(&original));
    assert!(FloatRules::dfa().is_equivalent(&original));
    assert!(float_rules::suffix().is_equivalent(&Dfa::string("f32").union(&Dfa::string("f64"))));
}

#[test]
fn float_literal_options_are_constants() {
    assert_eq!(RustFloat::OPTIONS.separator, Some('_'));
    assert_eq!(RustFloat::OPTIONS.suffixes, ["f32", "f64"]);
}

const DIGIT: RangeInclusive<char> = '0'..='9';

fsm! {
    /// Decimal integers with optional `_` separators, such as `1_000`.
    Integer {
        start: Start,
        Start { on: [(DIGIT, Digits)] },
        Digits { accept: true, on: [(DIGIT, Digits), ('_'..='_', Separator)] },
        /// After `_`; needs another digit.
        Separator { on: [(DIGIT, Digits)] },
    }
}

#[test]
fn fsm_transitions() {
    assert_eq!(Integer::START, Integer::Start);
    assert_eq!(
        Integer::STATES,
        [Integer::Start, Integer::Digits, Integer::Separator]
    );
    assert_eq!(Integer::Start.step('7'), Some(Integer::Digits));
    assert_eq!(Integer::Digits.step('_'), Some(Integer::Separator));
    assert_eq!(Integer::Separator.step('0'), Some(Integer::Digits));
}

#[test]
fn fsm_accepting_states() {
    assert!(!Integer::Start.is_accepting());
    assert!(Integer::Digits.is_accepting());
    assert!(!Integer::Separator.is_accepting());
    assert!(Integer::is_match("1_000"));
    assert!(!Integer::is_match("1_"));
    assert!(!Integer::is_match(""));
}

#[test]
fn fsm_missing_transitions() {
    assert_eq!(Integer::Start.step('_'), None);
    assert_eq!(Integer::Separator.step('_'), None);
    assert_eq!(Integer::Digits.step('x'), None);
    assert_eq!(Integer::run("1__0"), None);
    assert_eq!(Integer::run("1_"), Some(Integer::Separator));
}

#[test]
fn fsm_converts_to_an_equivalent_dfa() {
    let digit = Dfa::from_char_range('0'..='9');
    let expected = digit
        .plus()
        .concat(&Dfa::from_char('_').concat(&digit.plus()).star());
    let dfa = Integer::to_dfa();
    assert!(dfa.is_equivalent(&expected));
    // `Start` and `Separator` both need a digit next, so minimizing merges them;
    // with the dead state that leaves 3.
    assert_eq!(dfa.minimize().state_count(), 3);
}
