//! Tests for the plain-Rust pieces the declaration macros build on.

use std::{ops::RangeInclusive, sync::OnceLock};

use range_map_regex::{
    dfa::Dfa,
    fsm::{StateMachine, Table, is_disjoint},
    grammar::{FloatLiteralOptions, alt, chars, float_literal, lit, many, many1, one_of, opt, seq},
};

#[test]
fn from_transitions_handles_any_start_and_missing_transitions() {
    // State 2 is the start: "ab" then any number of "b".
    let dfa = Dfa::from_transitions(
        2,
        &[false, true, false],
        &[
            vec![('b'..='b', 1)],
            vec![('b'..='b', 1)],
            vec![('a'..='a', 0)],
        ],
    );
    assert!(dfa.is_match("ab"));
    assert!(dfa.is_match("abbb"));
    assert!(!dfa.is_match("a"));
    assert!(!dfa.is_match("ba"));
    assert!(!dfa.is_match(""));
    assert!(dfa.is_equivalent(&Dfa::string("ab").concat(&Dfa::from_char('b').star())));
}

#[test]
#[should_panic(expected = "overlapping transition ranges")]
fn from_transitions_rejects_overlap() {
    Dfa::from_transitions(0, &[true], &[vec![('a'..='m', 0), ('k'..='z', 0)]]);
}

#[test]
fn equivalence_and_emptiness() {
    let lower = Dfa::from_char_range('a'..='z');
    assert!(lower.intersection(&Dfa::from_char('A')).is_empty_language());
    assert!(!Dfa::<char>::epsilon().is_empty_language());
    assert!(lower.is_equivalent(&lower.minimize()));
    assert!(!lower.is_equivalent(&lower.star()));
}

#[test]
fn combinators_match_dfa_methods() {
    let digit = chars('0'..='9');
    let combinators = seq([
        one_of("+-"),
        many1(digit.clone()),
        opt(seq([lit("."), many(digit)])),
    ]);
    let methods = Dfa::from_char('+')
        .union(&Dfa::from_char('-'))
        .concat(&Dfa::from_char_range('0'..='9').plus())
        .concat(
            &Dfa::from_char('.')
                .concat(&Dfa::from_char_range('0'..='9').star())
                .optional(),
        );
    assert!(combinators.is_equivalent(&methods));
    assert!(alt([]).is_empty_language());
    assert!(seq([]).is_equivalent(&Dfa::epsilon()));
}

#[test]
fn float_literal_options() {
    let rust = float_literal(&FloatLiteralOptions::RUST);
    assert!(rust.is_match("12E+99_f64"));
    assert!(!rust.is_match("1__0"));

    let plain = float_literal(&FloatLiteralOptions {
        separator: None,
        exponent: false,
        suffixes: &[],
        ..FloatLiteralOptions::RUST
    });
    assert!(plain.is_match("1.5"));
    assert!(plain.is_match("2."));
    assert!(!plain.is_match("1_0.5"));
    assert!(!plain.is_match("1e5"));
    assert!(!plain.is_match("1.5f64"));
}

const BINARY: RangeInclusive<char> = '0'..='1';

/// Binary numbers with no leading zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Binary {
    Start,
    Zero,
    Number,
}

impl StateMachine for Binary {
    const START: Self = Binary::Start;
    const STATES: &'static [Self] = &[Binary::Start, Binary::Zero, Binary::Number];

    fn index(self) -> usize {
        self as usize
    }

    fn is_accepting(self) -> bool {
        self != Binary::Start
    }

    fn transitions(self) -> &'static [(RangeInclusive<char>, Self)] {
        use Binary::*;
        match self {
            Start => &[('0'..='0', Zero), ('1'..='1', Number)],
            Zero => &[],
            Number => &[(BINARY, Number)],
        }
    }

    fn table() -> &'static Table<Self> {
        static TABLE: OnceLock<Table<Binary>> = OnceLock::new();
        TABLE.get_or_init(Table::new)
    }
}

#[test]
fn hand_written_state_machine() {
    assert_eq!(Binary::Start.step('0'), Some(Binary::Zero));
    assert_eq!(Binary::Zero.step('0'), None);
    assert_eq!(Binary::Number.step('2'), None);
    assert_eq!(Binary::run("1011"), Some(Binary::Number));
    assert!(Binary::is_match("0"));
    assert!(!Binary::is_match("01"));
    assert!(!Binary::is_match(""));
    let dfa = Binary::to_dfa();
    assert!(dfa.is_match("1011") && !dfa.is_match("01"));
}

#[test]
fn disjointness() {
    const OK: &[(RangeInclusive<char>, ())] = &[('a'..='c', ()), ('d'..='d', ())];
    const OVERLAP: &[(RangeInclusive<char>, ())] = &[('a'..='c', ()), ('c'..='d', ())];
    const EMPTY: &[(RangeInclusive<char>, ())] = &[('z'..='a', ())];
    assert!(is_disjoint(OK));
    assert!(!is_disjoint(OVERLAP));
    assert!(!is_disjoint(EMPTY));
}
