//! Finite-state machines with named states, backed by [`RangeMapBlaze`].
//!
//! Implement [`StateMachine`] for a field-less enum whose variants are the states.
//! The trait then provides stepping, matching, and conversion to a [`Dfa`].
//!
//! ```rust,no_run
//! use std::{ops::RangeInclusive, sync::OnceLock};
//!
//! use range_map_regex::fsm::{StateMachine, Table};
//!
//! /// `[A-Za-z_][A-Za-z0-9_]*`, but not `_` alone.
//! #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
//! enum Ident {
//!     Start,
//!     Underscore,
//!     Word,
//! }
//!
//! impl StateMachine for Ident {
//!     const START: Self = Ident::Start;
//!     const STATES: &'static [Self] = &[Ident::Start, Ident::Underscore, Ident::Word];
//!
//!     fn index(self) -> usize {
//!         self as usize
//!     }
//!
//!     fn is_accepting(self) -> bool {
//!         self == Ident::Word
//!     }
//!
//!     fn transitions(self) -> &'static [(RangeInclusive<char>, Self)] {
//!         use Ident::*;
//!         match self {
//!             Start => &[('a'..='z', Word), ('A'..='Z', Word), ('_'..='_', Underscore)],
//!             Underscore | Word => &[
//!                 ('a'..='z', Word),
//!                 ('A'..='Z', Word),
//!                 ('0'..='9', Word),
//!                 ('_'..='_', Word),
//!             ],
//!         }
//!     }
//!
//!     fn table() -> &'static Table<Self> {
//!         static TABLE: OnceLock<Table<Ident>> = OnceLock::new();
//!         TABLE.get_or_init(Table::new)
//!     }
//! }
//!
//! assert!(Ident::is_match("_tmp"));
//! assert!(!Ident::is_match("_"));
//! assert_eq!(Ident::Start.step('_'), Some(Ident::Underscore));
//! assert_eq!(Ident::Start.step('1'), None);
//! ```

use std::{fmt::Debug, hash::Hash, ops::RangeInclusive};

use range_set_blaze::RangeMapBlaze;

use crate::dfa::Dfa;

/// A finite-state machine over `char` whose states are the values of `Self`.
///
/// See the [module example](self).
pub trait StateMachine: Copy + Eq + Hash + Debug + 'static {
    /// The state the machine starts in.
    const START: Self;
    /// Every state, in index order.
    const STATES: &'static [Self];

    /// This state's position in [`StateMachine::STATES`].
    fn index(self) -> usize;
    /// Whether input may end in this state.
    fn is_accepting(self) -> bool;
    /// This state's declared transitions. Characters not listed have no transition.
    fn transitions(self) -> &'static [(RangeInclusive<char>, Self)];
    /// The cached [`Table`] built from [`StateMachine::transitions`].
    fn table() -> &'static Table<Self>;

    /// The state after reading `symbol`, or `None` when there is no transition.
    /// See the [module example](self).
    fn step(self, symbol: char) -> Option<Self> {
        Self::table().maps[self.index()].get(symbol).copied()
    }

    /// The state after reading all of `input` from [`StateMachine::START`], or
    /// `None` when some character has no transition. See the [module example](self).
    fn run(input: &str) -> Option<Self> {
        input
            .chars()
            .try_fold(Self::START, |state, symbol| state.step(symbol))
    }

    /// Whether the machine accepts all of `input`. See the [module example](self).
    fn is_match(input: &str) -> bool {
        Self::run(input).is_some_and(Self::is_accepting)
    }

    /// Converts the machine to a [`Dfa`], for composition and minimization.
    /// See the [module example](self).
    fn to_dfa() -> Dfa {
        let accepting: Vec<bool> = Self::STATES
            .iter()
            .map(|state| state.is_accepting())
            .collect();
        let transitions: Vec<Vec<(RangeInclusive<char>, usize)>> = Self::STATES
            .iter()
            .map(|state| {
                state
                    .transitions()
                    .iter()
                    .map(|(range, target)| (range.clone(), target.index()))
                    .collect()
            })
            .collect();
        Dfa::from_transitions(Self::START.index(), &accepting, &transitions)
    }
}

/// One [`RangeMapBlaze`] per state, from character ranges to next states.
///
/// See the [module example](self).
pub struct Table<S: StateMachine> {
    maps: Vec<RangeMapBlaze<char, S>>,
}

impl<S: StateMachine> Table<S> {
    /// Builds the table from each state's [`StateMachine::transitions`].
    ///
    /// # Panics
    ///
    /// Panics if a state lists overlapping ranges or [`StateMachine::index`]
    /// disagrees with [`StateMachine::STATES`].
    pub fn new() -> Self {
        let maps = S::STATES
            .iter()
            .enumerate()
            .map(|(index, &state)| {
                assert_eq!(state.index(), index, "{state:?} has the wrong index");
                assert!(
                    is_disjoint(state.transitions()),
                    "{state:?} has overlapping transition ranges"
                );
                RangeMapBlaze::from_iter(state.transitions().iter().cloned())
            })
            .collect();
        Self { maps }
    }
}

impl<S: StateMachine> Default for Table<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether every range is non-empty and no two ranges overlap.
///
/// It is a `const fn` so declarations can check their transitions at compile time.
pub const fn is_disjoint<S>(transitions: &[(RangeInclusive<char>, S)]) -> bool {
    let mut i = 0;
    while i < transitions.len() {
        let (a_start, a_end) = (*transitions[i].0.start(), *transitions[i].0.end());
        if a_start > a_end {
            return false;
        }
        let mut j = i + 1;
        while j < transitions.len() {
            let (b_start, b_end) = (*transitions[j].0.start(), *transitions[j].0.end());
            if a_start <= b_end && b_start <= a_end {
                return false;
            }
            j += 1;
        }
        i += 1;
    }
    true
}
