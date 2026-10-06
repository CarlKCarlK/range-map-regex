//! Short combinators for writing [`Dfa`] grammars as expressions.
//!
//! Each function is a thin wrapper around a [`Dfa`] method, so grammars read like
//! regular expressions while still being ordinary Rust:
//!
//! ```rust,no_run
//! use range_map_regex::grammar::{chars, lit, many, many1, seq};
//!
//! let digit = chars('0'..='9');
//! let digits = seq([many1(digit.clone()), many(seq([lit("_"), many1(digit)]))]);
//! assert!(digits.is_match("1_000"));
//! assert!(!digits.is_match("1__0"));
//! ```

use std::ops::RangeInclusive;

use crate::dfa::Dfa;

/// Matches one character in `range`. See the [module example](self).
pub fn chars(range: RangeInclusive<char>) -> Dfa {
    Dfa::from_char_range(range)
}

/// Matches one character that appears in `set`, such as `one_of("eE")`.
/// See the [module example](self).
pub fn one_of(set: &str) -> Dfa {
    alt(set.chars().map(Dfa::from_char))
}

/// Matches exactly `text`. See the [module example](self).
pub fn lit(text: &str) -> Dfa {
    Dfa::string(text)
}

/// Matches each part in order. See the [module example](self).
pub fn seq(parts: impl IntoIterator<Item = Dfa>) -> Dfa {
    parts
        .into_iter()
        .fold(Dfa::epsilon(), |dfa, part| dfa.concat(&part))
}

/// Matches any one of the parts. See the [module example](self).
pub fn alt(parts: impl IntoIterator<Item = Dfa>) -> Dfa {
    parts
        .into_iter()
        .fold(Dfa::empty(), |dfa, part| dfa.union(&part))
}

/// Matches `part` or nothing. See the [module example](self).
pub fn opt(part: Dfa) -> Dfa {
    part.optional()
}

/// Matches zero or more repetitions of `part`. See the [module example](self).
pub fn many(part: Dfa) -> Dfa {
    part.star()
}

/// Matches one or more repetitions of `part`. See the [module example](self).
pub fn many1(part: Dfa) -> Dfa {
    part.plus()
}

/// Options for [`float_literal`].
///
/// ```rust,no_run
/// use range_map_regex::grammar::{FloatLiteralOptions, float_literal};
///
/// let no_exponent = float_literal(&FloatLiteralOptions {
///     exponent: false,
///     ..FloatLiteralOptions::RUST
/// });
/// assert!(no_exponent.is_match("1_000.5f64"));
/// assert!(!no_exponent.is_match("1e10"));
/// ```
#[derive(Debug, Clone)]
pub struct FloatLiteralOptions {
    /// Characters allowed as digits.
    pub digits: RangeInclusive<char>,
    /// Separator allowed between digits and before a suffix, such as `'_'`.
    pub separator: Option<char>,
    /// Whether an exponent such as `e10` or `E-3` is allowed.
    pub exponent: bool,
    /// Allowed type suffixes, such as `"f64"`.
    pub suffixes: &'static [&'static str],
}

impl FloatLiteralOptions {
    /// Rust-like float literals: `1_000.5`, `2.`, `1e10`, `12E+99_f64`, `5f32`.
    /// See [`FloatLiteralOptions`].
    pub const RUST: Self = Self {
        digits: '0'..='9',
        separator: Some('_'),
        exponent: true,
        suffixes: &["f32", "f64"],
    };
}

/// Builds a DFA for float literals described by `options`. See [`FloatLiteralOptions`].
///
/// A literal is digits followed by `.` (with optional digits, exponent, and suffix),
/// by an exponent (with optional suffix), or directly by a suffix.
pub fn float_literal(options: &FloatLiteralOptions) -> Dfa {
    let digit = chars(options.digits.clone());
    let digits = match options.separator {
        Some(separator) => seq([
            many1(digit.clone()),
            many(seq([Dfa::from_char(separator), many1(digit)])),
        ]),
        None => many1(digit),
    };
    let exponent = if options.exponent {
        seq([one_of("eE"), opt(one_of("+-")), digits.clone()])
    } else {
        Dfa::empty()
    };
    let suffix = alt(options.suffixes.iter().copied().map(lit));
    let separator = options.separator.map_or_else(Dfa::empty, Dfa::from_char);
    let optional_suffix = opt(seq([opt(separator), suffix.clone()]));

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
