# Experiment: `const-structures` declarations for `range-map-regex`

Branch `experiment/const-structures-dfa`. Not pushed. `main` is unchanged.

**Question.** Does `const-structures` make it meaningfully easier for a library
author to offer a pleasant, keyword-based API for finite-state machines, or does
ordinary Rust already express these applications better?

**Short answer.** The framework did its job well. All three macros compiled on
the first attempt, and their diagnostics were good without extra work. But for
this domain, ordinary Rust is as good or better in two of the three designs.
Regular languages are built by composing expressions, and Rust expressions
already compose. The one design that gives users something real is `fsm!`,
because an FSM really does have named items to generate (a state enum). Even
there the gain over hand-written Rust is modest, and the four-construct template
language makes the transition syntax awkward.

The biggest improvements came from four plain-Rust additions that need no macro:
`Dfa::from_transitions`, `Dfa::is_equivalent`, short combinators, and a
`StateMachine` trait.

**Recommendation:** use it only as a standalone demonstration of
`const-structures`, with `fsm!` as the example. Keep the plain-Rust additions
whatever is decided about the macros. Reasons are in [Recommendation](#10-recommendation).

## 0. Setup and repository state

- `~/programs/range-map-regex`: `main` matched `origin/main` (`aec4d35`) after
  `git fetch`, so there was nothing to fast-forward. The only local change was a
  regenerated `Cargo.lock`: the version of the local range-set-blaze checkout
  had changed from 0.5.0 to 0.7.0. The branch was created from `aec4d35` and
  carried that change along. It is now part of the branch's first commit.
- **The project did not build as checked out.** `Cargo.toml` uses
  `range-set-blaze = { path = "../range-set-blaze" }`, but that checkout is on
  its `gpu` branch. That branch has no `RangeMapBlaze::universe_with` and no
  `inner_join`, both of which `Dfa` needs. Those APIs exist only on
  range-set-blaze's `regex` branch (`976930ad`, the same commit locally and on
  `origin`). I left that checkout alone. In commit `cd6d00c` the experimental
  branch depends on
  `{ git = "https://github.com/CarlKCarlK/range-set-blaze", branch = "regex" }`.
  Revert this before merging anything if you prefer the path dependency.
- `const-structures`: used as-is from `../const-structures`, branch
  `publishing-readiness` (`f318291`), as an optional path dependency.
- Baseline with that dependency: library tests pass (3 pass, 1 already
  `#[ignore]`d). Four of the five examples run. `not_ident` passes all of its
  assertions but then fails inside Graphviz, because `dot` rejects an HTML label
  longer than 16 KB (the XID ranges). That failure was already there and is
  unrelated to this work.
- I ran the examples with stub `wslview` and `xdg-open` commands, so no SVG
  viewer windows opened.

## 1. What `range-map-regex` already provides

`Dfa<S: Integer = char>` (`src/dfa.rs`) has three parts: a start `StateId`, an
accepting or rejecting kind per state, and one `RangeMapBlaze<S, StateId>` per
state. Each map is *universal*: every symbol maps to a next state, and a dead
state absorbs rejected input.

- **Constructors:** `empty`, `epsilon`, `from_char`, `from_char_range`,
  `from_char_set`, `from_chars_where`, `string`, `xid_start`, `xid_continue`.
- **Operations:** `union` and `intersection` (lazy product construction),
  `complement` (flips the state kinds), `concat` and `star` (subset
  construction, built on demand with an `IndexMap` worklist), `optional`,
  `plus`, `minimize` (Moore-style partition refinement over range signatures),
  and `to_utf8_dfa`.
- **Matching:** `is_match`, `is_match_symbols`, and `is_match_bytes`.
- **Before this experiment,** a DFA could be built only from these
  combinators. Nothing public built one from an explicit state table, because
  `new`, `new_state`, and `set_transitions` are private.

`examples/float_literals.rs` builds a Rust-like float literal in 25 lines of
`Dfa` method calls. **Unminimized, the result has 3,908 states. Minimized, it
has 15.** Repeated `concat` and `union` of unminimized parts make the
intermediate DFAs blow up. This turned out to matter more than any syntax
question (see §6).

## 2. Plain-Rust additions (commit `0d28a32`, no `const-structures`)

Writing the experiment showed what the existing API lacked. All of these are
ordinary Rust and always compiled:

| Addition | Purpose | Lines |
| --- | --- | --- |
| `Dfa::from_transitions(start, &accepting, &transitions)` | Builds a DFA from an explicit table. Missing transitions go to a dead state. Panics on overlapping ranges. | ~80 |
| `Dfa::is_empty_language`, `Dfa::is_equivalent` | Exact language equality, used to check every approach against the original | ~30 |
| `#[derive(Clone)]` on `Dfa` | Lets rules and parts be reused | 1 |
| `grammar`: `chars`, `lit`, `one_of`, `seq`, `alt`, `opt`, `many`, `many1` | Regex-like expressions | ~60 |
| `grammar::float_literal` + `FloatLiteralOptions` (with `RUST` const) | Plain-Rust target for `float_literal!` | ~70 |
| `fsm::StateMachine` trait, `fsm::Table`, `fsm::is_disjoint` (`const fn`) | Named-state machines backed by one `RangeMapBlaze<char, State>` per state, with `step`, `run`, `is_match`, `to_dfa` | ~180 |

Tests: `tests/fsm_and_grammar.rs` (7 tests), which run without the feature.

## 3. The declaration layer (commit `c005c28`)

The library-author side is in `src/declare.rs`, behind the optional `declare`
feature (`declare = ["dep:const-structures"]`). The crate root re-exports
`__const_structures_expand` only when the feature is on. Without the feature,
nothing changes for users of the existing API.

Application-developer examples:

- `examples/declare_float_literals.rs`: the float literal written six ways.
- `examples/declare_identifier.rs`: an identifier written four ways.

Tests: `tests/declare.rs` (6 tests) and `tests/declare_ui.rs`, which runs
trybuild on 11 compile-fail cases in `tests/ui/`.

### Experiment A1: `float_literal!` (keyword options)

This is the sketch from the brief, made concrete.

```rust
float_literal! {
    /// Rust-like float literals, as in `float_literals.rs`.
    pub RustFloat {
        separator: '_',
        suffixes: ["f32", "f64"],
    }
}
float_literal! { pub NoExponent { separator: '_', exponent: false, suffixes: ["f32", "f64"] } }
float_literal! { pub PlainDecimal {} }
```

Schema: `digits: expr = '0'..='9'`, `separator?: expr`, `exponent: expr = true`,
`suffixes: expr = []`. The template generates a unit struct with
`const OPTIONS: FloatLiteralOptions`, a cached and minimized
`fn dfa() -> &'static Dfa`, and `fn is_match`. The library author wrote 64
lines, including docs and a doctest.

### Experiment A2: `dfa!` (named rules)

```rust
dfa! {
    /// Rust-like float literals, as named rules.
    pub FloatRules {
        start: Float,

        /// `1`, `1_000`: digits with single `_` separators.
        Digits { is: seq([many1(chars('0'..='9')), many(seq([lit("_"), many1(chars('0'..='9'))]))]) },
        /// `e10`, `E+9`, `e-2`.
        Exponent { is: seq([one_of("eE"), opt(one_of("+-")), digits()]) },
        /// `f32` or `f64`.
        Suffix { is: alt([lit("f32"), lit("f64")]) },
        /// Nothing, or a suffix with an optional `_` before it.
        OptionalSuffix { is: opt(seq([opt(lit("_")), suffix()])) },
        /// The literal itself.
        Float {
            is: alt([
                seq([digits(), lit("."), opt(digits()), opt(exponent()), optional_suffix()]),
                seq([digits(), exponent(), optional_suffix()]),
                seq([digits(), suffix()]),
            ]),
        },
    }
}
```

Each member is a rule. It becomes `pub fn snake_name() -> Dfa` in a generated
module `float_rules`. That module contains `use super::*;` and
`use range_map_regex::grammar::*;`. Each rule is built and minimized once, in a
`OnceLock`. The struct `FloatRules` gets `dfa()` (the `start` rule) and
`is_match`. Rules call each other by their `snake_case` names. The library
author wrote 76 lines.

### Experiment B: `fsm!` (explicit states)

```rust
const DIGIT: RangeInclusive<char> = '0'..='9';

fsm! {
    /// Rust-like float literals, as explicit states.
    pub FloatState {
        start: Start,

        Start { on: [(DIGIT, Int)] },
        /// After integer digits.
        Int { on: [(DIGIT, Int), ('_'..='_', IntSep), ('.'..='.', Dot), ('e'..='e', Exp), ('E'..='E', Exp), ('f'..='f', F)] },
        IntSep { on: [(DIGIT, Int)] },
        /// After `1.`
        Dot { accept: true, on: [(DIGIT, Frac), ('_'..='_', SuffixSep), ('e'..='e', Exp), ('E'..='E', Exp), ('f'..='f', F)] },
        Frac { accept: true, on: [(DIGIT, Frac), ('_'..='_', FracSep), ('e'..='e', Exp), ('E'..='E', Exp), ('f'..='f', F)] },
        /// `_` here may separate digits or come before a suffix.
        FracSep { on: [(DIGIT, Frac), ('f'..='f', F)] },
        Exp { on: [(DIGIT, ExpDigits), ('+'..='+', ExpSign), ('-'..='-', ExpSign)] },
        ExpSign { on: [(DIGIT, ExpDigits)] },
        ExpDigits { accept: true, on: [(DIGIT, ExpDigits), ('_'..='_', ExpSep), ('f'..='f', F)] },
        ExpSep { on: [(DIGIT, ExpDigits), ('f'..='f', F)] },
        SuffixSep { on: [('f'..='f', F)] },
        F { on: [('3'..='3', F3), ('6'..='6', F6)] },
        F3 { on: [('2'..='2', Done)] },
        F6 { on: [('4'..='4', Done)] },
        Done { accept: true },
    }
}

assert_eq!(FloatState::run("1_000."), Some(FloatState::Dot));
assert_eq!(FloatState::run("1__"), None); // `IntSep` has no `_` transition
```

Schema: `start: ident`, then `members 1.. { accept: expr = false, on: expr = [] }`.
The library author wrote 105 lines, 31 of them documentation.

### What `fsm!` generates

For the identifier example, the expansion is equivalent to:

```rust
/// Simplified identifiers, as explicit states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Ident {
    /// Member `Start` of `Ident`, generated by `fsm!`. | accept | false (default) | on | [(LOWER, Word), …] |
    Start,
    /// A lone `_` is not an identifier.     <- the user's doc replaces the table
    Underscore,
    Word,
}

impl Ident {
    const ON_START: &'static [(RangeInclusive<char>, Self)] = { use Ident::*; &[(LOWER, Word), (UPPER, Word), (UNDERSCORE, Underscore)] };
    // ... one per state
}

const _: () = assert!(range_map_regex::fsm::is_disjoint(Ident::ON_START),
                      concat!("state `", "Start", "` has overlapping or empty transition ranges"));
// ... one per state

impl range_map_regex::fsm::StateMachine for Ident {
    const START: Self = Self::Start;
    const STATES: &'static [Self] = &[Self::Start, Self::Underscore, Self::Word];
    fn index(self) -> usize { self as usize }
    fn is_accepting(self) -> bool { match self { Self::Start => false, Self::Underscore => false, Self::Word => true } }
    fn transitions(self) -> &'static [(RangeInclusive<char>, Self)] { match self { Self::Start => Self::ON_START, /* ... */ } }
    fn table() -> &'static Table<Self> { static TABLE: OnceLock<Table<Ident>> = OnceLock::new(); TABLE.get_or_init(Table::new) }
}
```

Two template techniques here are worth recording:

- **References between members.** A transition target such as `Word` sits
  inside an opaque `expr`. It resolves because the template wraps the value in
  `{ use $decl.name::*; &$state.on }`. `const-structures` never sees the state
  names inside `on`; rustc checks them.
- **Compile-time validation.** A template can emit `const _: () = assert!(…)`
  that calls a library `const fn`. The template language can't compute
  anything, but const evaluation can.

## 4. Results

### Correctness

Every version of each grammar was checked for **exact language equality** with
`Dfa::is_equivalent`, not just on the sample strings:

- Float: the original methods, combinators, combinators with minimized parts,
  `float_literal!`, `dfa!`, and `fsm!` all accept the same language. The
  original 12 valid and 11 invalid strings are asserted for each.
- Identifier: `Dfa` methods, a hand-written `StateMachine` enum, `dfa!`, and
  `fsm!` all accept the same language. For every test input, `fsm!` and the
  hand-written enum end in the same state.

### Source size (user side, non-blank, non-`//` lines, `///` docs counted)

| Grammar | `Dfa` methods | Combinators / hand-written | `float_literal!` | `dfa!` | `fsm!` |
| --- | ---: | ---: | ---: | ---: | ---: |
| Float literal | 25 | 18 (combinators) | 7* | 22 | 25 |
| Identifier | 10 | 33 (hand-written `StateMachine` enum) | — | 9 | 10 |

\* `float_literal!` is short only because the grammar lives in the library
(`grammar::float_literal`, about 35 lines). The ordinary-Rust equivalent is one
line: `float_literal(&FloatLiteralOptions { exponent: false, ..FloatLiteralOptions::RUST })`.

Three observations:

- Most of the readability gain over the original came from the **combinators**
  (25 → 18 lines, and the code reads like a regex). The combinators are
  ordinary Rust. `dfa!` (22 lines) is no shorter.
- The **one large reduction** is `fsm!` against a hand-written `StateMachine`
  enum: 10 lines against 33. The macro removes the `STATES` list, the `index`
  and `is_accepting` matches, and the `OnceLock` table cache.
- rustfmt doesn't format inside brace macros. The `fsm!` lines reach 126
  columns, and nothing wraps them.

### DFA build time (release, first use, median of 3 runs)

| Version | States built | Minimized | Time |
| --- | ---: | ---: | ---: |
| 1. `Dfa` methods | 3,908 | 15 | 45 ms |
| 2. Combinators | 3,908 | 15 | 51 ms |
| 2b. Combinators, `.minimize()` on each named part | 15 | 15 | **0.71 ms** |
| 3. `float_literal!` | 15 (minimized at end) | 15 | 53 ms |
| 4. `dfa!` (each rule minimized) | 15 | 15 | **0.71 ms** |
| 5. `fsm!` → `to_dfa()` | 16 | 15 | 0.02 ms |

`dfa!` looks 60× faster, but the 2b control shows the speedup comes from
**minimizing the named parts**, not from the macro. Minimizing each rule is a
policy the macro applies for you. A plain helper function could apply it too.
For `fsm!`, the user has done the "compilation" to states by hand, so building
the table is trivial.

### Compile time

These were measured with a separate target directory, using wall-clock times
from `cargo build` and `cargo check`.

| What | Without `declare` | With `declare` |
| --- | ---: | ---: |
| Clean `cargo check --lib`, including dependencies | 1.42 s | 3.10 s |
| Clean `cargo build --release --lib` | 2.32 s | 3.74 s |
| Rebuild of the lib after `touch` (release) | 0.93 s | 0.93–0.99 s |

The fixed cost is the `const-structures` proc macro and `syn` 3, about 1.4 to
1.7 s on a clean build. Defining the macros adds nothing measurable when the
library is rebuilt.

Per-declaration cost was measured with a scratch crate of 50 copies of the
identifier, timing a rebuild of one binary (3 runs each):

| 50 declarations | `cargo check` | `cargo build --release` |
| --- | ---: | ---: |
| `fsm!` | 0.23 s | 3.25 s |
| Hand-written `StateMachine` enums | 0.17 s | 3.21 s |
| `dfa!` | 0.19 s | 1.22 s |
| Plain cached combinator functions | 0.14 s | 0.72 s |

Expansion costs about 1 ms per declaration in `cargo check`. In release builds
`fsm!` costs the same as hand-written code. `dfa!` costs more in release because
it generates one cached function per rule, so there is more code to compile.
That cost comes from the generated code, not from expansion.

### Diagnostics (trybuild cases in `tests/ui/`)

| Mistake | What the user sees | Quality |
| --- | --- | --- |
| `Word { acept: true }` | ``unknown field `acept`; expected one of `accept`, `on` ``, at `acept` | Excellent (from the framework) |
| Duplicate state `Word {}` | ``duplicate member `Word` ``, at the second `Word` | Excellent (from the framework) |
| Target typo `Wrod` | ``cannot find value `Wrod` `` plus "a unit variant with a similar name exists: `Word`", at `Wrod` | Excellent (rustc, through the glob import) |
| `start: Begin` | ``no variant … named `Begin` found for enum `Ident` ``, at `Begin` | Good |
| `('_', Word)` instead of `('_'..='_', Word)` | ``expected `RangeInclusive<char>`, found `char` ``, at `'_'` | Good, but it exposes the awkward syntax |
| Overlapping ranges in a state | ``evaluation panicked: state `Start` has overlapping or empty transition ranges`` | Names the state, but **the span covers the whole macro call** |
| `float_literal! { exponent: "yes" }` | ``expected `bool`, found `&str` ``, at `"yes"` | Excellent |
| `dfa!` rule typo `digti()` | ``cannot find function `digti` `` plus a suggestion of `digit`, at `digti` | Excellent |
| `dfa!` `is: "[0-9]+"` | ``expected `Dfa`, found `&str` ``, at the string | Good |
| `dfa!` `start: Digts` | ``cannot find value `digts` in module `number` ``, with the suggestion **`start: digits`** | **Misleading**: the suggestion is wrong. The `$snake` renaming leaks, and the user must write `Digits`. |
| `dfa!` `start: Number` (the type's own name) | A confusing private-module-import error | Poor. The generated module name and the start rule collide. |

### Generated documentation

The macro pages get a generated **Syntax** block and **Fields** and
**Member fields** tables, all from the schema. Declared items get value tables.
For `fsm!`, each state's rustdoc lists its `accept` value and its `on`
transitions. That is genuinely useful: rustdoc becomes a readable state table.
For `dfa!`, each rule's function shows its `is` expression.

The catch comes from the framework's own rule: **a user's `///` on a state
replaces the table.** In the example, `Underscore` is documented, so its
transitions vanish from the docs. The framework offers no way to say "append
to the generated docs instead of replacing them".

## 5. Comparison

The three approaches are (1) ordinary Rust (`Dfa` methods or combinators, or a
hand-written `StateMachine` enum for explicit states), (2) declarative
composition (`float_literal!` and `dfa!`), and (3) explicit states (`fsm!`).

| Criterion | (1) Ordinary Rust | (2) `float_literal!` / `dfa!` | (3) `fsm!` |
| --- | --- | --- | --- |
| Readability | Combinators read like a regex. `let` names the parts. | `dfa!` reads the same as combinators, but the rules must be called by different (`snake_case`) names than they are declared with. `float_literal!` is clear, but so is a struct literal with `..RUST`. | Best for state-oriented thinking. Single-character ranges (`'.'..='.'`) and one tuple per range add noise. |
| Conciseness | Combinators 18 lines; hand-written FSM enum 33 | `dfa!` 22 (no gain). `float_literal!` moves the code into the library. | 10 vs 33 for the identifier: a real reduction of boilerplate |
| Expressiveness | Everything | `dfa!` can express everything combinators can (rules are expressions). `float_literal!` only covers the knobs the author chose. | Any DFA, but the user does subset construction by hand. Fine for 3 states, error-prone at 15 (my float FSM needed `FracSep` and `SuffixSep` to resolve the `_` ambiguity). |
| Error messages | rustc's, at the user's code | Mostly good. The `snake_case` leak and the module name collision are bad. | Very good, except the overlap check, whose span covers the whole macro |
| Documentation | Hand-written | Per-rule definition tables | Per-state transition tables, unless the user documents the state |
| Flexibility | Full | Rules are `Dfa`s, so they compose. `float_rules::exponent()` is reusable. | `to_dfa()` restores full composition: intersection with a keyword complement was tested. |
| Maintenance (library author) | Combinators ~60 lines | 64 + 76 lines of `define!`, plus support code | 105 lines of `define!`, plus the 180-line `fsm` module, which hand-written Rust uses too |
| Compilation | Baseline | +1.4–1.7 s clean (dependencies). `dfa!` generates more code. | +1.4–1.7 s clean. ~1 ms per declaration. Release cost unchanged. |
| Generality | — | Not embedded at all | Not embedded at all. A natural use: it generates items (enum and impls). |

## 6. Does generating named items help?

- **`float_literal!`: no.** A named unit struct with `OPTIONS` and `dfa()`
  doesn't beat a `FloatLiteralOptions` struct literal, which ordinary Rust
  already gives users. It has named fields, defaults through `..RUST`, and
  checked types. This is the pattern the `const-structures` README itself says
  to avoid: "for ordinary data, use Rust structs and constants". The generated
  rustdoc table is nice, but it doesn't justify a macro.
- **`dfa!`: barely.** The named items are functions that ordinary Rust users
  already write (`fn digits() -> Dfa`). Its real advantages are caching each
  rule and minimizing it, which is the 60× build-time difference. Both belong to
  a function such as `grammar::cached(|| …)`, not to a declaration language. Its
  costs are real: rules have two names (`Digits` declared, `digits()` called), a
  hidden module and `use super::*` scope, a misleading `start` diagnostic, and
  no rustfmt.
- **`fsm!`: yes, modestly.** An enum of states is genuinely an item, and Rust
  has no shorter way to declare an enum together with its transition table. The
  macro removes about 70% of the boilerplate, adds a compile-time determinism
  check, and documents each state's transitions. Generic code cannot do any of
  this, because generic functions can't own per-type statics or emit per-variant
  matches.

Ordinary Rust has a strong competitor for explicit FSMs: a `match (state, ch)`
function. It is idiomatic, rustc warns about unreachable arms, and it formats
cleanly. What `fsm!` adds over that is `RangeMapBlaze`-backed tables and
conversion to `Dfa`, which a `match` can't provide without probing all 1.1
million characters.

## 7. Limitations of the four-construct template language

1. **No nested repetition.** Members exist only at the top level, so a state
   can't have *transition members*. Transitions therefore live in one opaque
   `expr` (`on: [(range, Target), …]`). Each consequence below follows from
   that.
   - `const-structures` can't check the transitions, document them as a table,
     or validate target names. rustc checks the names, thanks to the
     `use Enum::*` glob trick.
   - One target per range, and a single character is written `'.'..='.'`.
     Array elements must share a type, so `('.', Dot)` next to
     `('0'..='9', Int)` can't type-check. The template can't wrap each element
     in `Into`, because it can't look inside an `expr`.
   - The natural syntax, `Int { '0'..='9' => Int, '_' => IntSep }`, is out of
     reach.
2. **No branching on values.** `$if let` tests only whether an optional field
   is present. `exponent: true` can't select template text. It has to become
   runtime or const Rust (`exponent: $decl.exponent` passed to a function).
   That is fine, and it is the framework's stated design.
3. **No computation**, so validation must go through `const fn`. This works
   (`const _: () = assert!(is_disjoint(…))`), but the error span is the whole
   invocation, because a template can't attach a user value's span to an
   assertion it generates.
4. **Identifier conversion leaks into diagnostics.** `$snake($decl.start)`
   produces `digts`. rustc then suggests `digits`, which is the wrong
   spelling for the caller to write.
5. **Cross-member references aren't modeled.** `start: Start` and every target
   inside `on` refer to other members. The framework has no "reference to a
   member" kind, so it can't say "`Begin` is not a state; expected one of
   `Start`, `Word`".
6. **Doc replacement is all-or-nothing.** Documenting a member hides its
   generated value table.

None of these needed a fifth construct to get a working result. Items 1 and 5
are what would be needed to make `fsm!` good rather than adequate.

## 8. Would either design be a convincing public demonstration?

- `float_literal!`: no. It shows the anti-pattern (a macro in place of a struct
  literal).
- `dfa!`: no. Readers will rightly ask why it isn't just functions.
- `fsm!`: **a reasonable but not a compelling demonstration.** In its favor:
  - It shows `const-structures` outside embedded Rust.
  - It generates real items: an enum, trait impls, and per-type statics.
  - It shows required, defaulted, and member fields.
  - The framework's own diagnostics are excellent.
  - It shows a compile-time check through `const fn`.
  - Its rustdoc doubles as a state table.

  Against it, the transition syntax shows the framework's weakest point (no
  nested repetition) on the first line a reader sees.
  `('.'..='.', Dot)` invites the reply "a `match` would be nicer". As a
  demonstration it works best when presented honestly, with the limitation
  stated.

## 9. What I'd keep regardless

These are improvements to `range-map-regex` itself, and none of them needs
`const-structures`:

- `Dfa::from_transitions`: the missing public constructor.
- `Dfa::is_equivalent` and `is_empty_language`: they made every comparison in
  this report exact.
- `grammar` combinators: the largest readability gain measured here.
- `StateMachine` and `Table`: named states with `RangeMapBlaze` tables and
  `to_dfa()`.
- The finding that **minimizing named sub-expressions turns a 45 ms,
  3,908-state construction into a 0.7 ms, 15-state one**. That is worth
  documenting, or doing automatically in combinators such as `seq` and `alt`.

## 10. Recommendation

**Use it only as a standalone demonstration of `const-structures`, with `fsm!`
as the example. Do not adopt `float_literal!` or `dfa!`.**

- Ordinary Rust already expresses composition-style DFAs as well as the macros
  do, or better. With combinators the float grammar takes 18 lines, against 22
  for `dfa!`. The only measured advantage of `dfa!`, per-rule minimization, is
  a library policy rather than a syntax.
- `fsm!` is the one place where a declaration that generates items pays off.
  It is 3× shorter than the hand-written equivalent, adds a compile-time
  determinism check, and documents each state's transitions. Even so, the
  current template language forces transitions into an opaque expression with
  awkward single-character ranges. I wouldn't make that syntax part of
  `range-map-regex`'s public API.
- **On the ultimate question:** `const-structures` made it easy for the
  *library author*. The three macros came to 245 lines including docs, needed
  no template debugging, and got schema-driven errors and docs for free. It
  does not make the *user's* API meaningfully better for DFAs, except for
  explicit FSMs, and there only modestly. That is a framework doing its job in
  a domain that mostly doesn't need it, not a framework failing. A domain whose
  members carry structured sub-lists, such as states with transitions, would
  show it off better after nested members exist.

## Files on the branch

| File | Role |
| --- | --- |
| `src/dfa.rs` (+124 lines) | `from_transitions`, `is_empty_language`, `is_equivalent`, `Clone` |
| `src/grammar.rs` | Combinators, `float_literal`, `FloatLiteralOptions` |
| `src/fsm.rs` | `StateMachine`, `Table`, `is_disjoint` |
| `src/declare.rs` | **Library author:** `float_literal!`, `dfa!`, `fsm!` (feature `declare`) |
| `examples/declare_float_literals.rs` | **Application developer:** float written six ways, with equivalence checks and timings |
| `examples/declare_identifier.rs` | **Application developer:** identifier written four ways, with stepping, missing transitions, composition |
| `tests/fsm_and_grammar.rs` | Plain-Rust API tests (no feature) |
| `tests/declare.rs` | Macro behavior: transitions, accepting states, missing transitions, equivalence |
| `tests/declare_ui.rs`, `tests/ui/*` | 11 compile-fail diagnostics |

Commands:

```text
cargo test --release --all-targets                     # existing API, no const-structures
cargo test --release --features declare --all-targets  # plus macros and UI tests
cargo run --release --features declare --example declare_float_literals
cargo run --release --features declare --example declare_identifier
```
