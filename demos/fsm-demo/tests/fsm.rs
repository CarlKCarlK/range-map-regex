//! `fsm!` behavior: transitions, accepting states, missing transitions, and
//! equivalence with ordinary `Dfa` construction.

use std::ops::RangeInclusive;

use fsm_demo::{StateMachine, fsm};
use range_map_regex::grammar::{chars, lit, many, many1, seq};

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
fn transitions() {
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
fn accepting_states() {
    assert!(!Integer::Start.is_accepting());
    assert!(Integer::Digits.is_accepting());
    assert!(!Integer::Separator.is_accepting());
    assert!(Integer::is_match("1_000"));
    assert!(!Integer::is_match("1_"));
    assert!(!Integer::is_match(""));
}

#[test]
fn missing_transitions() {
    assert_eq!(Integer::Start.step('_'), None);
    assert_eq!(Integer::Separator.step('_'), None);
    assert_eq!(Integer::Digits.step('x'), None);
    assert_eq!(Integer::run("1__0"), None);
    assert_eq!(Integer::run("1_"), Some(Integer::Separator));
}

#[test]
fn equivalent_to_composed_dfa() {
    let digit = chars('0'..='9');
    let composed = seq([many1(digit.clone()), many(seq([lit("_"), many1(digit)]))]);
    let dfa = Integer::to_dfa();
    assert!(dfa.is_equivalent(&composed));
    // `Start` and `Separator` both need a digit next, so minimizing merges them;
    // with the dead state that leaves 3.
    assert_eq!(dfa.minimize().state_count(), 3);
}
