# Experiment: `macro-schema` declarations for `range-map-regex`

<!-- todo0 consider deleting this spec once its conclusions are settled and the branch is merged or abandoned. -->

Branch `experiment/const-structures-dfa`.

The framework was named `const-structures` when this experiment ran; it has
since been renamed `macro-schema`. The branch keeps the old name.

**Question.** Does `macro-schema` make it meaningfully easier for a library
author to offer a pleasant, keyword-based API for finite-state machines, or does
ordinary Rust already express these applications better?

**Answer.** For composition-style regular languages, ordinary Rust is better. The
experiment's lasting improvements are plain Rust:

- exact language equivalence;
- minimizing grammar combinators;
- an explicit state-table constructor;
- a `StateMachine` trait.

One macro, `fsm!`, earned its place as a **demonstration** of `macro-schema`
outside embedded Rust. It lives in the self-contained crate `demos/fsm-demo`.
`range-map-regex` itself doesn't depend on `macro-schema`.

## What was kept

In `range-map-regex` (no `macro-schema`):

| API | Purpose |
| --- | --- |
| `Dfa::is_equivalent`, `Dfa::is_empty_language` | Exact language equality: each language minus the other is empty. Every comparison below uses this, not sample strings. |
| `grammar::{chars, one_of, lit, seq, alt, opt, many, many1}` | Regex-like combinators. Each returns a **minimized** DFA (see [Minimization](#minimization)). |
| `Dfa::from_transitions` | Builds a DFA from an explicit state table. Previously no public constructor took explicit states. Missing transitions go to a dead state, and overlapping ranges panic. |
| `fsm::{StateMachine, Table, is_disjoint}` | Named-state machines over `char`, backed by one `RangeMapBlaze` per state. The trait provides `step`, `run`, `is_match`, and `to_dfa`. Useful on its own when the states matter, for example to report where input stopped. |
| `#[derive(Clone)]` on `Dfa` | Lets parts be reused in combinator expressions |

The existing `Dfa` API is unchanged. Its methods still don't minimize.

In `demos/fsm-demo` (depends on `macro-schema` and `range-map-regex`):

- `src/lib.rs`: the library author's `fsm!`, a schema plus a template.
- `examples/identifier.rs`: one recognizer written three ways: grammar
  combinators, a hand-written `StateMachine`, and `fsm!`. All three are checked
  for exact equivalence and for agreeing state by state.
- `examples/float_literal.rs`: the float literal as 15 explicit states,
  checked for equivalence with the combinator version.
- `tests/fsm.rs` covers transitions, accepting states, missing transitions, and
  equivalence. `tests/ui/` holds six compile-fail diagnostics.

The demo is a workspace member, but not a default member. Plain `cargo build` and
`cargo test` touch only the library. `cargo check-all` (`--workspace`) also
tests the demo.

## Approaches tested

The float literal from `examples/float_literals.rs` and a simplified identifier
were each written several ways. Every version was verified to accept exactly
the same language.

| Approach | Syntax | Kept? |
| --- | --- | --- |
| `Dfa` methods (existing) | `digits.concat(&dot).concat(&digits.optional())…` | Yes (unchanged) |
| Grammar combinators | `seq([digits(), lit("."), opt(digits())…])` | **Yes** |
| `float_literal!` | `float_literal! { pub RustFloat { separator: '_', suffixes: ["f32", "f64"] } }` | No |
| `dfa!` | `dfa! { pub FloatRules { start: Float, Digits { is: … }, Float { is: … } } }` | No |
| Hand-written `StateMachine` enum | an enum plus a trait impl | Yes (trait in the library) |
| `fsm!` | `fsm! { Identifier { start: Start, Start { on: [(LOWER, Word), …] }, … } }` | **As a demo only** |

### Source size (user side, non-blank lines, `///` docs counted)

| Grammar | `Dfa` methods | Combinators | `float_literal!` | `dfa!` | Hand-written enum | `fsm!` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Float literal | 25 | 18 | 7* | 22 | — | 25 |
| Identifier | 10 | 8 | — | 9 | 33 | 10 |

\* Short only because the grammar lived in the library. The plain-Rust
equivalent was a one-line struct literal.

### Diagnostics

`macro-schema` reports schema errors at the user's token, for example
``unknown field `acept`; expected one of `accept`, `on` `` and
``duplicate member `Word` ``. For other mistakes, the generated code makes rustc
point at the user's token. Examples are a misspelled target state
(``cannot find value `Wrod` ``, with a suggestion of `Word`), a wrong `start`,
and a `char` where a range is needed.

Two diagnostics were weak:

- The `fsm!` overlap check names the state, but its span covers the whole
  invocation. A const-eval panic can't carry the user's span.
- In `dfa!`, a misspelled `start` produced the suggestion `start: digits`. The
  `$snake` renaming leaked into rustc's hint; the user must write `Digits`.

### Compile time

| What | Cost |
| --- | --- |
| `macro-schema` and `syn` 3 on a clean build | +1.4–1.7 s |
| Defining the macros, on a rebuild of the library | not measurable |
| 50 `fsm!` declarations vs 50 hand-written enums, `cargo check` | 0.23 s vs 0.17 s (~1 ms per declaration) |
| The same, `cargo build --release` | 3.25 s vs 3.21 s |

Moving `fsm!` into the demo removes the first cost for every user of the
library.

## Minimization

The unminimized float DFA built with `Dfa` methods has **3,908 states**, and
building it takes about 50 ms. Minimized, it has 15. The growth comes from
composing unminimized parts. It has nothing to do with macros: the `dfa!`
experiment first appeared 60× faster, but a control showed that the speedup
came entirely from minimizing each named part, which plain Rust gets by calling
`.minimize()`.

Before choosing a combinator policy, four variants were measured (release
builds, best of 5 runs):

| Variant | Float (states, time) | Identifier (states, time) |
| --- | --- | --- |
| No minimization | 3,908, 50.9 ms | 28, 0.28 ms |
| `.minimize()` on each named part | 15, 0.62 ms | — |
| Minimize after every fold step in `seq` and `alt` | 15, 0.60 ms | 4, 0.13 ms |
| **Minimize once per combinator call** | **15, 0.56 ms** | **4, 0.12 ms** |

Minimizing after every step hurt long sequences: a 44-character literal built
one character at a time was 35% slower. Minimizing once per call avoids that,
and `lit` minimizes once, so it is never quadratic. The combinators are new API,
so adopting "every combinator returns a minimized DFA" changed no existing
behavior. The language never changes; only state numbering and `state_count`
do. `Dfa`'s own methods stay unminimized, so existing state counts and
`display` output are unchanged.

## Why `float_literal!` and `dfa!` were rejected

- **`float_literal!`** was keyword options for one fixed grammar. Its generated
  struct, with `OPTIONS` and `dfa()`, was no better than an options struct
  literal with `..RUST`. That struct literal is ordinary Rust, with named fields,
  defaults, and type checking. This is the case the `macro-schema` README
  itself says to leave to plain structs.
- **`dfa!`** was named rules, each a cached, minimized `Dfa` function in a
  generated module. It was no shorter than the combinators (22 lines vs 18). Its
  only measured advantage, per-rule minimization, is a library policy, now built
  into the combinators. Its costs were real:
  - every rule had two names (`Digits` declared, `digits()` called);
  - its scope was a hidden `use super::*` module;
  - one diagnostic was misleading, and a `start` equal to the type's name
    produced a confusing error;
  - rustfmt doesn't format inside the macro.

## Why `fsm!` was kept as a demonstration

An explicit state machine genuinely declares items: an enum of states, its
trait implementation, and a cached table. Generic code can't produce these,
because generic functions can't own per-type statics or emit per-variant matches.
`fsm!` turns 33 lines of hand-written `StateMachine` into 10. It also checks
determinism at compile time (`const _: () = assert!(is_disjoint(…))`), and its
rustdoc lists each state's transitions.

It stays out of the library's API because of the tradeoff:

- **No nested repetition.** `macro-schema` allows members only at the top
  level of a declaration, so a state can't have transitions as members. Each
  state's transitions are one opaque expression, `on: [(range, Target), …]`.
  - Every element must have the same type, so single characters are written
    `'.'..='.'`, and each range needs its own tuple.
  - `macro-schema` can't check or tabulate transitions. rustc checks target
    names because the template wraps the value in `{ use Name::*; … }`.
  - A `match`-like syntax (`'0'..='9' => Digits`) isn't possible.
- **The user does subset construction by hand.** Three states is pleasant. The
  15-state float literal needed states such as `FractionSeparator` to resolve
  the ambiguity of `_`, which the combinators handle automatically.
- **Documentation is all-or-nothing.** A `///` comment on a state replaces its
  generated transition table.
- **rustfmt doesn't format the body.** Float-literal lines reach 126 columns.

Other limitations of the four-construct template language:

- Values can't drive branching, only the presence of optional fields.
- Validation has to go through `const fn`, so its errors have the invocation's
  span.
- There is no "reference to a member" field kind, so `start: Begin` can't be
  reported as "not a state".

None of these were worked around by extending `macro-schema`.

## Recommendation

- Keep the plain-Rust improvements. They make `range-map-regex` better whatever
  happens to declaration macros.
- Keep `fsm!` only as a standalone demonstration. It shows `macro-schema`
  generating items outside embedded Rust, with good diagnostics and generated
  documentation. It also shows the framework's main gap (nested members) on its
  first line. Present it with that limitation stated.
- Don't add declaration macros to `range-map-regex`'s API.

## Remaining issues

- **range-set-blaze dependency.** `range-map-regex` needs
  `RangeMapBlaze::universe_with` and the range-values `inner_join`, which
  exist only on range-set-blaze's `regex` branch. The local `../range-set-blaze`
  checkout is on a different branch, so `Cargo.toml` temporarily uses
  `{ git = "https://github.com/CarlKCarlK/range-set-blaze", branch = "regex" }`.
  Once those APIs are merged into range-set-blaze's `main`, return to the path
  dependency or a release.
- **The demo finds `macro-schema` through a local path.** Its dependency is
  `{ version = "0.1.0", path = "../../../macro-schema" }`, so it builds only
  next to a local `macro-schema` checkout.
  Once `macro-schema` is published, the path can be dropped.
- `examples/not_ident.rs` passes its assertions, then fails in Graphviz (`dot`
  rejects an HTML label over 16 KB). That failure predates this work.
