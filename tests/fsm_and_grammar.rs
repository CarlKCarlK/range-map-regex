//! Tests for `Dfa::from_transitions`, `Dfa::is_equivalent`, the `grammar`
//! combinators, and `StateMachine`.

use std::{ops::RangeInclusive, sync::OnceLock};

use range_map_regex::{
    dfa::Dfa,
    fsm::{StateMachine, Table, is_disjoint},
    grammar::{alt, chars, lit, many, many1, one_of, opt, seq},
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

/// The float literal from `examples/float_literals.rs`, built with `Dfa` methods.
fn float_literal_with_dfa_methods() -> Dfa {
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

#[test]
fn combinators_build_the_float_literal_minimized() {
    let digit = chars('0'..='9');
    let digits = seq([many1(digit.clone()), many(seq([lit("_"), many1(digit)]))]);
    let exponent = seq([one_of("eE"), opt(one_of("+-")), digits.clone()]);
    let suffix = alt([lit("f32"), lit("f64")]);
    let optional_suffix = opt(seq([opt(lit("_")), suffix.clone()]));
    let float = alt([
        seq([
            digits.clone(),
            lit("."),
            opt(digits.clone()),
            opt(exponent.clone()),
            optional_suffix.clone(),
        ]),
        seq([digits.clone(), exponent, optional_suffix]),
        seq([digits, suffix]),
    ]);

    let with_methods = float_literal_with_dfa_methods();
    assert!(float.is_equivalent(&with_methods));
    // The `Dfa` methods don't minimize, so composition grows large; the combinators
    // minimize every result.
    assert_eq!(with_methods.state_count(), 3908);
    assert_eq!(with_methods.minimize().state_count(), 15);
    assert_eq!(float.state_count(), 15);
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
