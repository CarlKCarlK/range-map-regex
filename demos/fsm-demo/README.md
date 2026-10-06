# fsm-demo

A demonstration of [`macro-schema`](https://crates.io/crates/macro-schema) outside
embedded Rust. It is not part of `range-map-regex`'s API, and `range-map-regex`
doesn't depend on `macro-schema`.

- **Library author:** `src/lib.rs` defines `fsm!` with a schema and a template.
  The template generates an enum of states and an implementation of
  `range_map_regex::fsm::StateMachine`.
- **Application developer:** `examples/identifier.rs` writes the same
  recognizer three ways (grammar combinators, a hand-written `StateMachine`, and
  `fsm!`) and checks that all three accept exactly the same language.
  `examples/float_literal.rs` declares a 15-state float literal.

```rust
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
```

## Tradeoff

`fsm!` replaces about 33 lines of hand-written `StateMachine` code with 10. It
also checks at compile time that a state's ranges don't overlap, and gives each
state rustdoc listing its transitions.

But the `macro-schema` template language allows repeated members only at the
top level, so transitions can't be nested members of a state. They are one
expression per state, `on: [(range, Target), ...]`. Every element must have the
same type, so single characters are written `'.'..='.'`, and a `match`-like
syntax (`'0'..='9' => Digits`) isn't possible.

See `specs/MACRO_SCHEMA_EXPERIMENT.md` in the parent repository for the
measurements and the approaches that were rejected.

## Running

From the `range-map-regex` root:

```text
cargo test --release -p fsm-demo
cargo run --release -p fsm-demo --example identifier
cargo run --release -p fsm-demo --example float_literal
```
