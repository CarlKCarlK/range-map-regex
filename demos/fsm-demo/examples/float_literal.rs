//! The Rust-like float literal from range-map-regex's `examples/float_literals.rs`,
//! declared as 15 explicit states and checked against ordinary `Dfa` construction.
//!
//! It shows where `fsm!` gets awkward: the author does the subset construction by
//! hand (`FractionSeparator` exists only because `_` after fraction digits may start
//! either more digits or a suffix), and single characters need `'.'..='.'`.
//!
//! Run with `cargo run --release -p fsm-demo --example float_literal`.

use std::ops::RangeInclusive;

use fsm_demo::{StateMachine, fsm};
use range_map_regex::{
    dfa::Dfa,
    grammar::{alt, chars, lit, many, many1, one_of, opt, seq},
};

const VALID: [&str; 12] = [
    "123.0f64",
    "0.1f64",
    "12E+99_f64",
    "5f32",
    "2.",
    "1e10",
    "1.e10",
    "2E+9f64",
    "3.14e-2",
    "1_000.0",
    "1_000f32",
    "10f64",
];

const INVALID: [&str; 11] = [
    "-1.0", "2e", "0x80.0", "f64", ".5", "5", "1__0", "1._", "1._0", "1e+", "10f16",
];

fn main() {
    for valid in VALID {
        assert!(Float::is_match(valid), "expected valid: {valid}");
    }
    for invalid in INVALID {
        assert!(!Float::is_match(invalid), "expected invalid: {invalid}");
    }
    assert_eq!(Float::run("1_000."), Some(Float::Dot));
    assert_eq!(Float::run("1_000_"), Some(Float::IntegerSeparator));
    assert_eq!(Float::run("1__"), None); // `IntegerSeparator` has no `_` transition

    let composed = float_literal_dfa();
    assert!(Float::to_dfa().is_equivalent(&composed));
    println!(
        "fsm! states: {}, composed DFA states: {}, same language",
        Float::STATES.len(),
        composed.state_count()
    );
    println!("float_literal passed!");
}

fn float_literal_dfa() -> Dfa {
    let digit = chars('0'..='9');
    let digits = seq([many1(digit.clone()), many(seq([lit("_"), many1(digit)]))]);
    let exponent = seq([one_of("eE"), opt(one_of("+-")), digits.clone()]);
    let suffix = alt([lit("f32"), lit("f64")]);
    let optional_suffix = opt(seq([opt(lit("_")), suffix.clone()]));
    alt([
        seq([
            digits.clone(),
            lit("."),
            opt(digits.clone()),
            opt(exponent.clone()),
            optional_suffix.clone(),
        ]),
        seq([digits.clone(), exponent, optional_suffix]),
        seq([digits, suffix]),
    ])
}

const DIGIT: RangeInclusive<char> = '0'..='9';

fsm! {
    /// Rust-like float literals, as explicit states.
    Float {
        start: Start,

        Start { on: [(DIGIT, Integer)] },
        /// After integer digits.
        Integer { on: [(DIGIT, Integer), ('_'..='_', IntegerSeparator), ('.'..='.', Dot), ('e'..='e', Exponent), ('E'..='E', Exponent), ('f'..='f', F)] },
        IntegerSeparator { on: [(DIGIT, Integer)] },
        /// After `1.`
        Dot { accept: true, on: [(DIGIT, Fraction), ('_'..='_', SuffixSeparator), ('e'..='e', Exponent), ('E'..='E', Exponent), ('f'..='f', F)] },
        Fraction { accept: true, on: [(DIGIT, Fraction), ('_'..='_', FractionSeparator), ('e'..='e', Exponent), ('E'..='E', Exponent), ('f'..='f', F)] },
        /// `_` here may separate digits or come before a suffix.
        FractionSeparator { on: [(DIGIT, Fraction), ('f'..='f', F)] },
        Exponent { on: [(DIGIT, ExponentDigits), ('+'..='+', ExponentSign), ('-'..='-', ExponentSign)] },
        ExponentSign { on: [(DIGIT, ExponentDigits)] },
        ExponentDigits { accept: true, on: [(DIGIT, ExponentDigits), ('_'..='_', ExponentSeparator), ('f'..='f', F)] },
        ExponentSeparator { on: [(DIGIT, ExponentDigits), ('f'..='f', F)] },
        SuffixSeparator { on: [('f'..='f', F)] },
        F { on: [('3'..='3', F3), ('6'..='6', F6)] },
        F3 { on: [('2'..='2', Done)] },
        F6 { on: [('4'..='4', Done)] },
        Done { accept: true },
    }
}
