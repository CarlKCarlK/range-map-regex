//! A simplified identifier, `[A-Za-z_][A-Za-z0-9_]*` but not `_` alone, written four ways.
//!
//! 1. Ordinary Rust with `Dfa` methods.
//! 2. Ordinary Rust: a hand-written enum implementing `StateMachine`.
//! 3. `dfa!`: named rules.
//! 4. `fsm!`: explicit named states and transitions.
//!
//! Run with `cargo run --release --features declare --example declare_identifier`.

use std::{ops::RangeInclusive, sync::OnceLock};

use range_map_regex::{
    declare::{dfa, fsm},
    dfa::Dfa,
    fsm::{StateMachine, Table},
};

const VALID: [&str; 6] = ["x", "hello", "_tmp", "Var123", "snake_case_2", "__"];
const INVALID: [&str; 6] = ["", "_", "1abc", "a-b", "éclair", "a b"];

fn main() {
    let composed = identifier_dfa();
    let dfas = [
        ("1. Dfa methods", composed.clone()),
        ("2. hand-written enum", HandIdent::to_dfa()),
        ("3. dfa! rules", IdentRules::dfa().clone()),
        ("4. fsm! states", Ident::to_dfa()),
    ];
    for (label, dfa) in &dfas {
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
            "{label:<22} minimized states: {}",
            dfa.minimize().state_count()
        );
    }

    // Named states give a stepping API that a composed `Dfa` doesn't have.
    assert_eq!(Ident::START, Ident::Start);
    assert_eq!(Ident::Start.step('_'), Some(Ident::Underscore));
    assert_eq!(Ident::Start.step('q'), Some(Ident::Word));
    assert_eq!(Ident::Underscore.step('7'), Some(Ident::Word));
    assert_eq!(Ident::Start.step('7'), None); // missing transition
    assert_eq!(Ident::Word.step('-'), None);
    assert!(Ident::Word.is_accepting());
    assert!(!Ident::Underscore.is_accepting());
    assert_eq!(Ident::run("_"), Some(Ident::Underscore));
    assert_eq!(Ident::run("a-b"), None);
    assert_eq!(
        Ident::STATES,
        [Ident::Start, Ident::Underscore, Ident::Word]
    );

    // The hand-written enum behaves identically, state for state.
    for input in VALID.iter().chain(&INVALID) {
        assert_eq!(
            Ident::run(input).map(Ident::index),
            HandIdent::run(input).map(HandIdent::index),
            "{input}"
        );
    }

    // A state machine converts to a `Dfa`, so the existing operations still apply.
    let keyword = Dfa::string("fn").union(&Dfa::string("let"));
    let non_keyword = Ident::to_dfa().intersection(&keyword.complement());
    assert!(non_keyword.is_match("lets"));
    assert!(!non_keyword.is_match("fn"));

    println!("declare_identifier passed!");
}

// ----- 1. Ordinary Rust with `Dfa` methods -----

fn identifier_dfa() -> Dfa {
    let letter = Dfa::from_char_range('a'..='z').union(&Dfa::from_char_range('A'..='Z'));
    let underscore = Dfa::from_char('_');
    let rest = letter
        .union(&Dfa::from_char_range('0'..='9'))
        .union(&underscore);
    letter
        .concat(&rest.star())
        .union(&underscore.concat(&rest.plus()))
}

// ----- 2. Ordinary Rust: a hand-written `StateMachine` -----

const LOWER: RangeInclusive<char> = 'a'..='z';
const UPPER: RangeInclusive<char> = 'A'..='Z';
const DIGIT: RangeInclusive<char> = '0'..='9';
const UNDERSCORE: RangeInclusive<char> = '_'..='_';

/// Hand-written equivalent of the `fsm!` declaration below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HandIdent {
    Start,
    Underscore,
    Word,
}

impl StateMachine for HandIdent {
    const START: Self = HandIdent::Start;
    const STATES: &'static [Self] = &[HandIdent::Start, HandIdent::Underscore, HandIdent::Word];

    fn index(self) -> usize {
        self as usize
    }

    fn is_accepting(self) -> bool {
        self == HandIdent::Word
    }

    fn transitions(self) -> &'static [(RangeInclusive<char>, Self)] {
        use HandIdent::*;
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
        static TABLE: OnceLock<Table<HandIdent>> = OnceLock::new();
        TABLE.get_or_init(Table::new)
    }
}

// ----- 3. `dfa!`: named rules -----

dfa! {
    /// Simplified identifiers, as named rules.
    IdentRules {
        start: Ident,

        Letter { is: alt([chars('a'..='z'), chars('A'..='Z')]) },
        Rest { is: alt([letter(), chars('0'..='9'), lit("_")]) },
        Ident { is: alt([seq([letter(), many(rest())]), seq([lit("_"), many1(rest())])]) },
    }
}

// ----- 4. `fsm!`: explicit states -----

fsm! {
    /// Simplified identifiers, as explicit states.
    Ident {
        start: Start,

        Start { on: [(LOWER, Word), (UPPER, Word), (UNDERSCORE, Underscore)] },
        /// A lone `_` is not an identifier.
        Underscore { on: [(LOWER, Word), (UPPER, Word), (DIGIT, Word), (UNDERSCORE, Word)] },
        Word { accept: true, on: [(LOWER, Word), (UPPER, Word), (DIGIT, Word), (UNDERSCORE, Word)] },
    }
}
