//! The Rust-like float literal from `float_literals.rs`, written five ways.
//!
//! 1. Ordinary Rust with `Dfa` methods (copied from `float_literals.rs`).
//! 2. Ordinary Rust with the `grammar` combinators.
//! 3. `float_literal!`: keyword options for one fixed grammar.
//! 4. `dfa!`: named rules.
//! 5. `fsm!`: explicit named states and transitions.
//!
//! Run with `cargo run --release --features declare --example declare_float_literals`.

use std::{ops::RangeInclusive, time::Instant};

use range_map_regex::{
    declare::{dfa, float_literal, fsm},
    dfa::Dfa,
    fsm::StateMachine,
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
    let start = Instant::now();
    let original = rust_like_float_literal_dfa();
    report("1. Dfa methods", &original, start);

    let start = Instant::now();
    let combinators = float_literal_with_combinators();
    report("2. grammar combinators", &combinators, start);

    // Control: the same combinators, minimizing each named part, as `dfa!` does.
    let start = Instant::now();
    let minimized_parts = float_literal_with_minimized_parts();
    report("2b. ... minimized parts", &minimized_parts, start);

    let start = Instant::now();
    let options = RustFloat::dfa();
    report("3. float_literal!", options, start);

    let start = Instant::now();
    let rules = FloatRules::dfa();
    report("4. dfa! rules", rules, start);

    let start = Instant::now();
    let states = FloatState::to_dfa();
    report("5. fsm! states", &states, start);

    for (name, dfa) in [
        ("combinators", &combinators),
        ("minimized parts", &minimized_parts),
        ("float_literal!", options),
        ("dfa!", rules),
        ("fsm!", &states),
    ] {
        assert!(
            dfa.is_equivalent(&original),
            "{name} differs from the original"
        );
    }
    println!("All six accept exactly the same language.");

    // The `fsm!` version can also be run directly, without building a `Dfa`.
    for valid in VALID {
        assert!(FloatState::is_match(valid), "expected valid: {valid}");
    }
    for invalid in INVALID {
        assert!(
            !FloatState::is_match(invalid),
            "expected invalid: {invalid}"
        );
    }
    assert_eq!(FloatState::run("1_000."), Some(FloatState::Dot));
    assert_eq!(FloatState::run("1_000_"), Some(FloatState::IntSep));
    assert_eq!(FloatState::run("1__"), None); // `IntSep` has no `_` transition

    // `float_literal!` turns options on and off by keyword.
    assert!(NoExponent::is_match("1_000.5f64"));
    assert!(!NoExponent::is_match("1e10"));
    assert!(PlainDecimal::is_match("1.5"));
    assert!(!PlainDecimal::is_match("1_000.5"));
    assert!(!PlainDecimal::is_match("1.5f64"));

    // `dfa!` rules are ordinary functions, so they can be reused and composed.
    let exponent = float_rules::exponent();
    assert!(exponent.is_match("E-1_0"));
    assert!(!exponent.is_match("E-"));
    let signed = opt(lit("-")).concat(rules);
    assert!(signed.is_match("-1.5e3"));

    println!("declare_float_literals passed!");
}

fn report(label: &str, dfa: &Dfa, start: Instant) {
    let elapsed = start.elapsed();
    for valid in VALID {
        assert!(dfa.is_match(valid), "{label}: expected valid: {valid}");
    }
    for invalid in INVALID {
        assert!(
            !dfa.is_match(invalid),
            "{label}: expected invalid: {invalid}"
        );
    }
    println!(
        "{label:<24} states: {:>5}, minimized: {:>3}, built in {elapsed:.2?}",
        dfa.state_count(),
        dfa.minimize().state_count()
    );
}

// ----- 1. Ordinary Rust with `Dfa` methods (from `float_literals.rs`) -----

fn rust_like_float_literal_dfa() -> Dfa {
    let digits = digits_with_underscores_dfa();

    let dot = Dfa::from_char('.');
    let exponent_marker = Dfa::from_char('e').union(&Dfa::from_char('E'));
    let exponent_sign = Dfa::from_char('+').union(&Dfa::from_char('-')).optional();
    let exponent = exponent_marker.concat(&exponent_sign).concat(&digits);

    let float_suffix = Dfa::string("f32").union(&Dfa::string("f64"));
    let float_suffix_with_optional_sep = Dfa::from_char('_').optional().concat(&float_suffix);
    let optional_suffix = float_suffix_with_optional_sep.optional();

    let decimal_with_dot = digits
        .concat(&dot)
        .concat(&digits.optional())
        .concat(&exponent.optional())
        .concat(&optional_suffix);

    let decimal_with_exponent = digits.concat(&exponent).concat(&optional_suffix);
    let integer_with_float_suffix = digits.concat(&float_suffix);

    decimal_with_dot
        .union(&decimal_with_exponent)
        .union(&integer_with_float_suffix)
}

fn digits_with_underscores_dfa() -> Dfa {
    let digit = Dfa::from_char_range('0'..='9');
    let underscore_then_digits = Dfa::from_char('_').concat(&digit.plus());
    digit.plus().concat(&underscore_then_digits.star())
}

// ----- 2. Ordinary Rust with the `grammar` combinators -----

fn float_literal_with_combinators() -> Dfa {
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

fn float_literal_with_minimized_parts() -> Dfa {
    let digit = chars('0'..='9');
    let digits = seq([many1(digit.clone()), many(seq([lit("_"), many1(digit)]))]).minimize();
    let exponent = seq([one_of("eE"), opt(one_of("+-")), digits.clone()]).minimize();
    let suffix = alt([lit("f32"), lit("f64")]).minimize();
    let optional_suffix = opt(seq([opt(lit("_")), suffix.clone()])).minimize();
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
    .minimize()
}

// ----- 3. `float_literal!`: keyword options -----

float_literal! {
    /// Rust-like float literals, as in `float_literals.rs`.
    pub RustFloat {
        separator: '_',
        suffixes: ["f32", "f64"],
    }
}

float_literal! {
    pub NoExponent { separator: '_', exponent: false, suffixes: ["f32", "f64"] }
}

float_literal! {
    pub PlainDecimal {}
}

// ----- 4. `dfa!`: named rules -----

dfa! {
    /// Rust-like float literals, as named rules.
    pub FloatRules {
        start: Float,

        /// `1`, `1_000`: digits with single `_` separators.
        Digits { is: seq([many1(chars('0'..='9')), many(seq([lit("_"), many1(chars('0'..='9'))]))]) },
        /// `e10`, `E+9`, `e-2`.
        Exponent { is: seq([one_of("eE"), opt(one_of("+-")), digits()]) },
        /// `f32` or `f64`.
        Suffix { is: alt([lit("f32"), lit("f64")]) },
        /// Nothing, or a suffix with an optional `_` before it.
        OptionalSuffix { is: opt(seq([opt(lit("_")), suffix()])) },
        /// The literal itself.
        Float {
            is: alt([
                seq([digits(), lit("."), opt(digits()), opt(exponent()), optional_suffix()]),
                seq([digits(), exponent(), optional_suffix()]),
                seq([digits(), suffix()]),
            ]),
        },
    }
}

// ----- 5. `fsm!`: explicit states -----

const DIGIT: RangeInclusive<char> = '0'..='9';

fsm! {
    /// Rust-like float literals, as explicit states.
    pub FloatState {
        start: Start,

        Start { on: [(DIGIT, Int)] },
        /// After integer digits.
        Int { on: [(DIGIT, Int), ('_'..='_', IntSep), ('.'..='.', Dot), ('e'..='e', Exp), ('E'..='E', Exp), ('f'..='f', F)] },
        IntSep { on: [(DIGIT, Int)] },
        /// After `1.`
        Dot { accept: true, on: [(DIGIT, Frac), ('_'..='_', SuffixSep), ('e'..='e', Exp), ('E'..='E', Exp), ('f'..='f', F)] },
        Frac { accept: true, on: [(DIGIT, Frac), ('_'..='_', FracSep), ('e'..='e', Exp), ('E'..='E', Exp), ('f'..='f', F)] },
        /// `_` here may separate digits or come before a suffix.
        FracSep { on: [(DIGIT, Frac), ('f'..='f', F)] },
        Exp { on: [(DIGIT, ExpDigits), ('+'..='+', ExpSign), ('-'..='-', ExpSign)] },
        ExpSign { on: [(DIGIT, ExpDigits)] },
        ExpDigits { accept: true, on: [(DIGIT, ExpDigits), ('_'..='_', ExpSep), ('f'..='f', F)] },
        ExpSep { on: [(DIGIT, ExpDigits), ('f'..='f', F)] },
        SuffixSep { on: [('f'..='f', F)] },
        F { on: [('3'..='3', F3), ('6'..='6', F6)] },
        F3 { on: [('2'..='2', Done)] },
        F6 { on: [('4'..='4', Done)] },
        Done { accept: true },
    }
}
