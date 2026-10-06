//! One identifier recognizer written three ways, checked for exact equivalence:
//!
//! 1. Ordinary `Dfa` construction with the `grammar` combinators.
//! 2. A hand-written enum implementing `StateMachine`.
//! 3. An `fsm!` declaration.
//!
//! The language is `[A-Za-z_][A-Za-z0-9_]*`, but not `_` alone.
//!
//! Run with `cargo run --release -p fsm-demo --example identifier`.

use std::{ops::RangeInclusive, sync::OnceLock};

use fsm_demo::{StateMachine, fsm};
use range_map_regex::{
    dfa::Dfa,
    fsm::Table,
    grammar::{alt, chars, lit, many, many1, seq},
};

const VALID: [&str; 6] = ["x", "hello", "_tmp", "Var123", "snake_case_2", "__"];
const INVALID: [&str; 6] = ["", "_", "1abc", "a-b", "éclair", "a b"];

fn main() {
    let composed = identifier_dfa();
    for (label, dfa) in [
        ("1. grammar combinators", composed.clone()),
        ("2. hand-written enum", HandWritten::to_dfa()),
        ("3. fsm! declaration", Identifier::to_dfa()),
    ] {
        for valid in VALID {
            assert!(dfa.is_match(valid), "{label}: expected valid: {valid}");
        }
        for invalid in INVALID {
            assert!(
                !dfa.is_match(invalid),
                "{label}: expected invalid: {invalid}"
            );
        }
        assert!(dfa.is_equivalent(&composed), "{label} differs");
        println!(
            "{label:<24} minimized states: {}",
            dfa.minimize().state_count()
        );
    }

    // Named states give a stepping API that a composed `Dfa` doesn't have.
    assert_eq!(Identifier::Start.step('_'), Some(Identifier::Underscore));
    assert_eq!(Identifier::Underscore.step('7'), Some(Identifier::Word));
    assert_eq!(Identifier::Start.step('7'), None); // missing transition
    assert!(!Identifier::Underscore.is_accepting());
    assert_eq!(Identifier::run("_"), Some(Identifier::Underscore));
    assert_eq!(Identifier::run("a-b"), None);

    // The declaration and the hand-written enum agree state for state.
    for input in VALID.iter().chain(&INVALID) {
        assert_eq!(
            Identifier::run(input).map(Identifier::index),
            HandWritten::run(input).map(HandWritten::index),
            "{input}"
        );
    }

    println!("identifier passed!");
}

// ----- 1. Ordinary `Dfa` construction -----

fn identifier_dfa() -> Dfa {
    let letter = alt([chars('a'..='z'), chars('A'..='Z')]);
    let rest = alt([letter.clone(), chars('0'..='9'), lit("_")]);
    alt([
        seq([letter, many(rest.clone())]),
        seq([lit("_"), many1(rest)]),
    ])
}

// ----- 2. A hand-written `StateMachine` -----

const LOWER: RangeInclusive<char> = 'a'..='z';
const UPPER: RangeInclusive<char> = 'A'..='Z';
const DIGIT: RangeInclusive<char> = '0'..='9';
const UNDERSCORE: RangeInclusive<char> = '_'..='_';

/// The hand-written equivalent of the `fsm!` declaration below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HandWritten {
    Start,
    Underscore,
    Word,
}

impl StateMachine for HandWritten {
    const START: Self = HandWritten::Start;
    const STATES: &'static [Self] = &[
        HandWritten::Start,
        HandWritten::Underscore,
        HandWritten::Word,
    ];

    fn index(self) -> usize {
        self as usize
    }

    fn is_accepting(self) -> bool {
        self == HandWritten::Word
    }

    fn transitions(self) -> &'static [(RangeInclusive<char>, Self)] {
        use HandWritten::*;
        match self {
            Start => &[(LOWER, Word), (UPPER, Word), (UNDERSCORE, Underscore)],
            Underscore | Word => &[
                (LOWER, Word),
                (UPPER, Word),
                (DIGIT, Word),
                (UNDERSCORE, Word),
            ],
        }
    }

    fn table() -> &'static Table<Self> {
        static TABLE: OnceLock<Table<HandWritten>> = OnceLock::new();
        TABLE.get_or_init(Table::new)
    }
}

// ----- 3. An `fsm!` declaration -----

fsm! {
    /// Simplified identifiers, as explicit states.
    Identifier {
        start: Start,

        Start { on: [(LOWER, Word), (UPPER, Word), (UNDERSCORE, Underscore)] },
        /// A lone `_` is not an identifier.
        Underscore { on: [(LOWER, Word), (UPPER, Word), (DIGIT, Word), (UNDERSCORE, Word)] },
        Word { accept: true, on: [(LOWER, Word), (UPPER, Word), (DIGIT, Word), (UNDERSCORE, Word)] },
    }
}
