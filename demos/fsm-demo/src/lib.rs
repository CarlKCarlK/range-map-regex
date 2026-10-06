//! A demonstration of `const-structures` outside embedded Rust: an `fsm!` macro
//! that declares a finite-state machine for [`range_map_regex`].
//!
//! This crate is the *library author's* side. It defines [`fsm!`] with a schema
//! and a template. The template generates an enum of the states and an
//! implementation of [`StateMachine`]; the matching, stepping, and conversion to a
//! [`Dfa`](range_map_regex::dfa::Dfa) all stay in `range_map_regex::fsm`.
//!
//! The *application developer's* side is in this crate's `examples/` directory.
//! `examples/identifier.rs` compares `fsm!` with a hand-written [`StateMachine`]
//! and with ordinary [`Dfa`](range_map_regex::dfa::Dfa) construction.
//!
//! # Tradeoff
//!
//! `fsm!` removes the boilerplate of a hand-written [`StateMachine`]: the `STATES`
//! list, the `index` and `is_accepting` matches, and the cached table. It also checks
//! at compile time that each state's ranges don't overlap, and it documents each
//! state's transitions in rustdoc.
//!
//! But `const-structures` allows repeated members only at the top level of a
//! declaration, so a state can't contain its transitions as nested members. Each
//! state's transitions are one opaque expression, `on: [(range, Target), ...]`:
//!
//! - every element must have the same type, so a single character is written
//!   `'.'..='.'` and each range gets its own tuple;
//! - the macro can't validate target names itself; rustc does, because the template
//!   brings the states into scope with `use Name::*`;
//! - a `match`-like syntax such as `'0'..='9' => Digits` is out of reach.
//!
//! For that reason `fsm!` is a demonstration, not part of `range_map_regex`'s API.
//! The experiment that led here is described in
//! `specs/CONST_STRUCTURES_EXPERIMENT.md` in the `range-map-regex` repository.

pub use range_map_regex::fsm::StateMachine;

// `const-structures` macros expand through this path.
#[doc(hidden)]
pub use const_structures::expand as __const_structures_expand;

/// Items the generated code names through `$crate`.
#[doc(hidden)]
pub mod __private {
    pub use range_map_regex::fsm::{Table, is_disjoint};
}

const_structures::define! {
    /// Declares a finite-state machine as an enum of its named states.
    ///
    /// Each member `Name { accept: BOOL, on: [(RANGE, Target), ...] }` is a state.
    /// Inside `on`, the states are in scope by name. Characters a state doesn't list
    /// have no transition, so the input is rejected. Overlapping or empty ranges in
    /// one state are a compile-time error.
    ///
    /// The enum implements [`StateMachine`], which provides `step`, `run`,
    /// `is_match`, and `to_dfa`. Transitions are stored as one `RangeMapBlaze` per
    /// state.
    ///
    /// ```rust,no_run
    /// use fsm_demo::{StateMachine, fsm};
    ///
    /// fsm! {
    ///     /// `[A-Za-z_][A-Za-z0-9_]*`, but not `_` alone.
    ///     pub Identifier {
    ///         start: Start,
    ///         Start { on: [('a'..='z', Word), ('A'..='Z', Word), ('_'..='_', Underscore)] },
    ///         Underscore { on: [('a'..='z', Word), ('A'..='Z', Word), ('0'..='9', Word), ('_'..='_', Word)] },
    ///         Word { accept: true, on: [('a'..='z', Word), ('A'..='Z', Word), ('0'..='9', Word), ('_'..='_', Word)] },
    ///     }
    /// }
    ///
    /// assert!(Identifier::is_match("_tmp"));
    /// assert_eq!(Identifier::Start.step('_'), Some(Identifier::Underscore));
    /// assert_eq!(Identifier::run("_"), Some(Identifier::Underscore));
    /// assert!(Identifier::to_dfa().is_match("x1"));
    /// ```
    pub fsm {
        /// The state the machine starts in.
        start: ident,

        /// Each member is a state.
        members 1.. {
            /// Whether input may end in this state.
            accept: expr = false,
            /// Outgoing transitions, as `[(range, Target), ...]`.
            on: expr = [],
        }
    }

    generate {
        $decl.attrs
        #[doc = $decl.doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $decl.vis enum $decl.name {
            $for state in $decl.members {
                $state.attrs
                #[doc = $state.doc]
                $state.name,
            }
        }

        impl $decl.name {
            $for state in $decl.members {
                const $upper(on_, $state.name): &'static [(::core::ops::RangeInclusive<char>, Self)] = {
                    use $decl.name::*;
                    &$state.on
                };
            }
        }

        $for state in $decl.members {
            const _: () = ::core::assert!(
                $crate::__private::is_disjoint($decl.name::$upper(on_, $state.name)),
                concat!(
                    "state `",
                    stringify!($state.name),
                    "` has overlapping or empty transition ranges"
                )
            );
        }

        impl $crate::StateMachine for $decl.name {
            const START: Self = Self::$decl.start;
            const STATES: &'static [Self] = &[$for state in $decl.members { Self::$state.name, }];

            fn index(self) -> usize {
                self as usize
            }

            fn is_accepting(self) -> bool {
                match self {
                    $for state in $decl.members {
                        Self::$state.name => $state.accept,
                    }
                }
            }

            fn transitions(self) -> &'static [(::core::ops::RangeInclusive<char>, Self)] {
                match self {
                    $for state in $decl.members {
                        Self::$state.name => Self::$upper(on_, $state.name),
                    }
                }
            }

            fn table() -> &'static $crate::__private::Table<Self> {
                static TABLE: ::std::sync::OnceLock<$crate::__private::Table<$decl.name>> =
                    ::std::sync::OnceLock::new();
                TABLE.get_or_init($crate::__private::Table::new)
            }
        }
    }
}
