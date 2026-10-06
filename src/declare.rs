//! Experimental keyword-based declarations, built with `const-structures`.
//!
//! Enabled by the `declare` feature. Each macro here is a schema plus a template;
//! the DFA work stays in [`Dfa`](crate::dfa::Dfa), [`grammar`](crate::grammar), and
//! [`fsm`](crate::fsm), which work without this module.
//!
//! - [`float_literal!`]: a float-literal type configured by keyword options.
//! - [`dfa!`]: a type matching a language built from named rules.
//! - [`fsm!`]: an enum of named states with declared transitions.

const_structures::define! {
    /// Declares a type that matches float literals, configured by keyword options.
    ///
    /// The type gets `OPTIONS`, a cached minimized `dfa()`, and `is_match`. It is the
    /// keyword form of [`float_literal`](crate::grammar::float_literal) with
    /// [`FloatLiteralOptions`](crate::grammar::FloatLiteralOptions).
    ///
    /// ```rust,no_run
    /// use range_map_regex::declare::float_literal;
    ///
    /// float_literal! {
    ///     /// Rust-like float literals.
    ///     pub RustFloat {
    ///         separator: '_',
    ///         suffixes: ["f32", "f64"],
    ///     }
    /// }
    ///
    /// assert!(RustFloat::is_match("12E+99_f64"));
    /// assert!(!RustFloat::is_match("1__0"));
    /// ```
    pub float_literal {
        /// Characters allowed as digits.
        digits: expr = '0'..='9',
        /// Separator allowed between digits and before a suffix, such as `'_'`.
        separator?: expr,
        /// Whether an exponent such as `e10` or `E-3` is allowed.
        exponent: expr = true,
        /// Allowed type suffixes, such as `["f32", "f64"]`.
        suffixes: expr = [],
    }

    generate {
        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name;

        impl $decl.name {
            /// The options this type was declared with.
            pub const OPTIONS: $crate::grammar::FloatLiteralOptions =
                $crate::grammar::FloatLiteralOptions {
                    digits: $decl.digits,
                    separator: $if let Some(separator) = $decl.separator {
                        ::core::option::Option::Some($separator)
                    } else {
                        ::core::option::Option::None
                    },
                    exponent: $decl.exponent,
                    suffixes: &$decl.suffixes,
                };

            /// The minimized DFA, built on first use.
            pub fn dfa() -> &'static $crate::dfa::Dfa {
                static DFA: ::std::sync::OnceLock<$crate::dfa::Dfa> = ::std::sync::OnceLock::new();
                DFA.get_or_init(|| $crate::grammar::float_literal(&Self::OPTIONS).minimize())
            }

            /// Whether all of `input` is a literal of this kind.
            pub fn is_match(input: &str) -> bool {
                Self::dfa().is_match(input)
            }
        }
    }
}

const_structures::define! {
    /// Declares a type that matches a regular language built from named rules.
    ///
    /// Each rule `Name { is: EXPR }` becomes a function `name()` in a generated module
    /// named after the type. `EXPR` is an ordinary expression of type
    /// [`Dfa`](crate::dfa::Dfa). It can call the [`grammar`](crate::grammar)
    /// combinators, the other rules by their `snake_case` names, and items in the
    /// enclosing module. Each rule is built and minimized once, on first use.
    ///
    /// The type gets `dfa()`, which is the `start` rule, and `is_match`.
    ///
    /// ```rust,no_run
    /// use range_map_regex::declare::dfa;
    ///
    /// dfa! {
    ///     /// `[A-Za-z_][A-Za-z0-9_]*`, but not `_` alone.
    ///     pub Identifier {
    ///         start: Ident,
    ///         Letter { is: alt([chars('a'..='z'), chars('A'..='Z')]) },
    ///         Rest { is: alt([letter(), chars('0'..='9'), lit("_")]) },
    ///         Ident { is: alt([seq([letter(), many(rest())]), seq([lit("_"), many1(rest())])]) },
    ///     }
    /// }
    ///
    /// assert!(Identifier::is_match("_tmp"));
    /// assert!(identifier::letter().is_match("q"));
    /// ```
    pub dfa {
        /// The rule whose language the type matches.
        start: ident,

        /// Each member is a named rule.
        members 1.. {
            /// The rule's definition, an expression of type `Dfa`.
            is: expr,
        }
    }

    generate {
        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name;

        impl $decl.name {
            /// The minimized DFA of the `start` rule, built on first use.
            pub fn dfa() -> &'static $crate::dfa::Dfa {
                static DFA: ::std::sync::OnceLock<$crate::dfa::Dfa> = ::std::sync::OnceLock::new();
                DFA.get_or_init($snake($decl.name)::$snake($decl.start))
            }

            /// Whether all of `input` matches the `start` rule.
            pub fn is_match(input: &str) -> bool {
                Self::dfa().is_match(input)
            }
        }

        #[doc = concat!("The rules of [`", stringify!($decl.name), "`], one function per rule.")]
        $decl.vis mod $snake($decl.name) {
            use super::*;
            use $crate::grammar::*;

            $for rule in $decl.members {
                $rule.attrs
                #[doc = $rule.doc]
                pub fn $snake($rule.name)() -> $crate::dfa::Dfa {
                    static DFA: ::std::sync::OnceLock<$crate::dfa::Dfa> = ::std::sync::OnceLock::new();
                    DFA.get_or_init(|| {
                        let rule: $crate::dfa::Dfa = $rule.is;
                        rule.minimize()
                    })
                    .clone()
                }
            }
        }
    }
}

const_structures::define! {
    /// Declares a finite-state machine as an enum of its named states.
    ///
    /// Each member `Name { accept: BOOL, on: [(RANGE, Target), ...] }` is a state.
    /// Inside `on`, the states are in scope by name. Characters a state doesn't list
    /// have no transition, so the input is rejected. Overlapping or empty ranges in
    /// one state are a compile-time error.
    ///
    /// The enum implements [`StateMachine`](crate::fsm::StateMachine), which provides
    /// `step`, `run`, `is_match`, and `to_dfa`. Transitions are stored as one
    /// `RangeMapBlaze` per state.
    ///
    /// ```rust,no_run
    /// use range_map_regex::{declare::fsm, fsm::StateMachine};
    ///
    /// fsm! {
    ///     /// `[A-Za-z_][A-Za-z0-9_]*`, but not `_` alone.
    ///     pub Ident {
    ///         start: Start,
    ///         Start { on: [('a'..='z', Word), ('A'..='Z', Word), ('_'..='_', Underscore)] },
    ///         Underscore { on: [('a'..='z', Word), ('A'..='Z', Word), ('0'..='9', Word), ('_'..='_', Word)] },
    ///         Word { accept: true, on: [('a'..='z', Word), ('A'..='Z', Word), ('0'..='9', Word), ('_'..='_', Word)] },
    ///     }
    /// }
    ///
    /// assert!(Ident::is_match("_tmp"));
    /// assert_eq!(Ident::Start.step('_'), Some(Ident::Underscore));
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
                $crate::fsm::is_disjoint($decl.name::$upper(on_, $state.name)),
                concat!(
                    "state `",
                    stringify!($state.name),
                    "` has overlapping or empty transition ranges"
                )
            );
        }

        impl $crate::fsm::StateMachine for $decl.name {
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

            fn table() -> &'static $crate::fsm::Table<Self> {
                static TABLE: ::std::sync::OnceLock<$crate::fsm::Table<$decl.name>> =
                    ::std::sync::OnceLock::new();
                TABLE.get_or_init($crate::fsm::Table::new)
            }
        }
    }
}
