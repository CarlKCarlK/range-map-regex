use std::io;

use range_map_regex::dfa::Dfa;
use range_map_regex::display::display_byte;

fn main() {
    if let Err(err) = inner_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn inner_main() -> io::Result<()> {
    let float_char = rust_like_float_literal_dfa();
    let minimized_char = float_char.minimize();

    let byte_dfa = minimized_char.to_utf8_dfa();
    let minimized_byte = byte_dfa.minimize();

    println!(
        "Char DFA states: before = {}, after = {}",
        float_char.state_count(),
        minimized_char.state_count()
    );
    println!(
        "Byte DFA states: before = {}, after = {}",
        byte_dfa.state_count(),
        minimized_byte.state_count()
    );

    display_byte(&minimized_byte)?;
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
