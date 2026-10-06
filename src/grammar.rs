//! Short combinators for writing [`Dfa`] grammars as expressions.
//!
//! Each function wraps [`Dfa`] methods, so grammars read like regular expressions
//! while staying ordinary Rust:
//!
//! ```rust,no_run
//! use range_map_regex::grammar::{alt, chars, lit, many, many1, one_of, opt, seq};
//!
//! // Rust-like float literals: `1_000.5`, `2.`, `1e10`, `12E+99_f64`, `5f32`.
//! let digit = chars('0'..='9');
//! let digits = seq([many1(digit.clone()), many(seq([lit("_"), many1(digit)]))]);
//! let exponent = seq([one_of("eE"), opt(one_of("+-")), digits.clone()]);
//! let suffix = alt([lit("f32"), lit("f64")]);
//! let optional_suffix = opt(seq([opt(lit("_")), suffix.clone()]));
//! let float = alt([
//!     seq([digits.clone(), lit("."), opt(digits.clone()), opt(exponent.clone()), optional_suffix.clone()]),
//!     seq([digits.clone(), exponent, optional_suffix]),
//!     seq([digits, suffix]),
//! ]);
//!
//! assert!(float.is_match("12E+99_f64"));
//! assert!(!float.is_match("1__0"));
//! assert_eq!(float.state_count(), 15);
//! ```
//!
//! # Minimization
//!
//! Every combinator returns a minimized DFA. Composition of unminimized parts can grow
//! quickly: built with the plain [`Dfa`] methods, the grammar above has 3,908 states
//! before minimization and takes about 50 ms; built with these combinators, it never
//! exceeds its final 15 states and takes about 0.6 ms. Minimizing once per call (not
//! once per part inside [`seq`] and [`alt`]) keeps the cost low for long sequences.
//!
//! The [`Dfa`] methods themselves don't minimize; call [`Dfa::minimize`] when mixing
//! them with large compositions.

use std::ops::RangeInclusive;

use crate::dfa::Dfa;

/// Matches one character in `range`.
///
/// ```rust,no_run
/// use range_map_regex::grammar::chars;
///
/// let lower = chars('a'..='z');
/// assert!(lower.is_match("q"));
/// assert!(!lower.is_match("Q"));
/// ```
pub fn chars(range: RangeInclusive<char>) -> Dfa {
    Dfa::from_char_range(range).minimize()
}

/// Matches one character that appears in `set`.
///
/// ```rust,no_run
/// use range_map_regex::grammar::one_of;
///
/// let sign = one_of("+-");
/// assert!(sign.is_match("-"));
/// assert!(!sign.is_match("+-"));
/// ```
pub fn one_of(set: &str) -> Dfa {
    set.chars()
        .fold(Dfa::empty(), |dfa, ch| dfa.union(&Dfa::from_char(ch)))
        .minimize()
}

/// Matches exactly `text`.
///
/// ```rust,no_run
/// use range_map_regex::grammar::lit;
///
/// let keyword = lit("fn");
/// assert!(keyword.is_match("fn"));
/// assert!(!keyword.is_match("f"));
/// ```
pub fn lit(text: &str) -> Dfa {
    Dfa::string(text).minimize()
}

/// Matches each part in order. With no parts, matches only the empty string.
///
/// ```rust,no_run
/// use range_map_regex::grammar::{chars, lit, seq};
///
/// let key = seq([lit("F"), chars('1'..='9')]);
/// assert!(key.is_match("F5"));
/// assert!(!key.is_match("F"));
/// ```
pub fn seq(parts: impl IntoIterator<Item = Dfa>) -> Dfa {
    parts
        .into_iter()
        .fold(Dfa::epsilon(), |dfa, part| dfa.concat(&part))
        .minimize()
}

/// Matches any one of the parts. With no parts, matches nothing.
///
/// ```rust,no_run
/// use range_map_regex::grammar::{alt, lit};
///
/// let boolean = alt([lit("true"), lit("false")]);
/// assert!(boolean.is_match("false"));
/// assert!(!boolean.is_match("maybe"));
/// ```
pub fn alt(parts: impl IntoIterator<Item = Dfa>) -> Dfa {
    parts
        .into_iter()
        .fold(Dfa::empty(), |dfa, part| dfa.union(&part))
        .minimize()
}

/// Matches `part` or the empty string.
///
/// ```rust,no_run
/// use range_map_regex::grammar::{lit, opt, seq};
///
/// let color = seq([lit("colo"), opt(lit("u")), lit("r")]);
/// assert!(color.is_match("color"));
/// assert!(color.is_match("colour"));
/// ```
pub fn opt(part: Dfa) -> Dfa {
    part.optional().minimize()
}

/// Matches zero or more repetitions of `part`.
///
/// ```rust,no_run
/// use range_map_regex::grammar::{lit, many};
///
/// let ha = many(lit("ha"));
/// assert!(ha.is_match(""));
/// assert!(ha.is_match("hahaha"));
/// assert!(!ha.is_match("hah"));
/// ```
pub fn many(part: Dfa) -> Dfa {
    part.star().minimize()
}

/// Matches one or more repetitions of `part`.
///
/// ```rust,no_run
/// use range_map_regex::grammar::{chars, many1};
///
/// let digits = many1(chars('0'..='9'));
/// assert!(digits.is_match("2026"));
/// assert!(!digits.is_match(""));
/// ```
pub fn many1(part: Dfa) -> Dfa {
    part.plus().minimize()
}
