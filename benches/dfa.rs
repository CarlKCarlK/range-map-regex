//! Dependency-free timing baseline; run with `cargo bench --bench dfa`.
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

use range_map_regex::{
    dfa::Dfa,
    grammar::{alt, chars, lit, many, many1, one_of, opt, seq},
};

fn main() {
    // Warm the Unicode caches: construction timings below represent subsequent builds.
    let start = Instant::now();
    black_box((Dfa::xid_start(), Dfa::xid_continue()));
    println!(
        "Unicode XID initialization: {:?} (one observation)",
        start.elapsed()
    );

    let identifier_raw = build_identifier();
    let identifier = identifier_raw.minimize();
    let float_raw = float_literal();
    let float = float_raw.minimize();
    println!(
        "Identifier states: {} -> {}",
        identifier_raw.state_count(),
        identifier.state_count()
    );
    println!(
        "Float states: {} -> {}",
        float_raw.state_count(),
        float.state_count()
    );

    measure("construct/identifier/warm", || {
        black_box(build_identifier());
    });
    measure("construct/float", || {
        black_box(float_literal());
    });
    measure("minimize/identifier", || {
        black_box(identifier_raw.minimize());
    });
    measure("minimize/float", || {
        black_box(float_raw.minimize());
    });

    let ascii = "identifier_123".repeat(8192);
    let unicode = "变量_Δé123".repeat(8192);
    let early_failure = format!("!{ascii}");
    let late_failure = format!("{ascii}!");
    for (name, input, expected) in [
        ("match/identifier/short", "hello_123", true),
        ("match/identifier/short_rejected", "!hello", false),
        ("match/identifier/ascii", ascii.as_str(), true),
        ("match/identifier/unicode", unicode.as_str(), true),
        (
            "match/identifier/early_failure",
            early_failure.as_str(),
            false,
        ),
        (
            "match/identifier/late_failure",
            late_failure.as_str(),
            false,
        ),
    ] {
        assert_eq!(identifier.is_match(input), expected);
        println!("{name}: {} input bytes", input.len());
        measure(name, || {
            black_box(identifier.is_match(black_box(input)));
        });
        measure(&format!("{name}/full_scan"), || {
            black_box(identifier.is_match_symbols(black_box(input).chars()));
        });
    }
    let long_float = format!("{}.0e+99_f64", "1234567890".repeat(8192));
    assert!(float.is_match(&long_float));
    measure("match/float/long", || {
        black_box(float.is_match(black_box(&long_float)));
    });
    measure("match/float/long/full_scan", || {
        black_box(float.is_match_symbols(black_box(&long_float).chars()));
    });
    for (name, input, expected) in [
        ("match/float/short", "12E+99_f64", true),
        ("match/float/rejected", "1__0", false),
    ] {
        assert_eq!(float.is_match(input), expected);
        measure(name, || {
            black_box(float.is_match(black_box(input)));
        });
        measure(&format!("{name}/full_scan"), || {
            black_box(float.is_match_symbols(black_box(input).chars()));
        });
    }
}

fn measure(name: &str, mut operation: impl FnMut()) {
    for _ in 0..3 {
        operation();
    }
    let start = Instant::now();
    let mut iterations = 0_u64;
    loop {
        for _ in 0..16 {
            operation();
        }
        iterations += 16;
        if start.elapsed() >= Duration::from_millis(300) {
            break;
        }
    }
    println!(
        "{name}: {:.0} ns/op ({iterations} iterations)",
        start.elapsed().as_nanos() as f64 / iterations as f64
    );
}

fn build_identifier() -> Dfa {
    let start = Dfa::xid_start();
    let continuation = Dfa::xid_continue();
    start
        .concat(&continuation.star())
        .union(&Dfa::from_char('_').concat(&continuation.plus()))
}

fn float_literal() -> Dfa {
    let digit = chars('0'..='9');
    let digits = seq([many1(digit.clone()), many(seq([lit("_"), many1(digit)]))]);
    let exponent = seq([one_of("eE"), opt(one_of("+-")), digits.clone()]);
    let suffix = alt([lit("f32"), lit("f64")]);
    let optional_suffix = opt(seq([opt(lit("_")), suffix.clone()]));
    // Minimized parts, but leave the final unions unminimized to measure that stage separately.
    let dotted = seq([
        digits.clone(),
        lit("."),
        opt(digits.clone()),
        opt(exponent.clone()),
        optional_suffix.clone(),
    ]);
    let exponential = seq([digits.clone(), exponent, optional_suffix]);
    let suffixed = seq([digits, suffix]);
    dotted.union(&exponential).union(&suffixed)
}
