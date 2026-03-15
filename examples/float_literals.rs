use std::io;

use range_map_regex::dfa::Dfa;
use range_map_regex::display::display_dfa;

fn main() {
    if let Err(err) = inner_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn inner_main() -> io::Result<()> {
    // This example is intentionally "Rust-like" rather than a full lexer clone.
    let float_literal = rust_like_float_literal_dfa();

    for valid in [
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
    ] {
        assert!(float_literal.is_match(valid), "expected valid: {valid}");
    }

    for invalid in [
        "-1.0",   // unary minus is separate from the literal token
        "2e",     // exponent must have digits
        "0x80.0", // non-decimal radices are not float literals
        "f64",
        ".5",
        "5",
        "1__0",
        "1._",
        "1._0",
        "1e+",
        "10f16",
    ] {
        assert!(!float_literal.is_match(invalid), "expected invalid: {invalid}");
    }

    let before_states = float_literal.state_count();
    let minimized = float_literal.minimize();
    let after_states = minimized.state_count();
    println!("DFA states: before minimize = {before_states}, after minimize = {after_states}");
    display_dfa(&minimized)?;

    println!("Rust-like float literal showcase passed!");
    Ok(())
}

fn rust_like_float_literal_dfa() -> Dfa {
    let digits = digits_with_underscores_dfa();

    let dot = Dfa::from_char('.');
    let exponent_marker = Dfa::from_char('e').union(&Dfa::from_char('E'));
    let exponent_sign = Dfa::from_char('+')
        .union(&Dfa::from_char('-'))
        .optional();
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
