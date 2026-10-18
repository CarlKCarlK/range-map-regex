Experiments in regex/dfa using RangeSetMaps (from range-set-map crate).
All licensing is the same as that crate.

Run the performance baseline with:

```sh
cargo bench --bench dfa
```

The dependency-free harness reports nanoseconds per operation for grammar construction,
minimization, and matching. It also reports state counts before and after minimization.
Identifier construction uses warmed Unicode caches; first-time XID initialization is
reported separately as a single observation. Float construction uses minimized grammar
parts and leaves the final unions unminimized, so final minimization is timed separately.

Matching uses minimized DFAs and includes long ASCII and Unicode identifiers, early and
late failures, and short and long float literals. Each matching case also times a
`/full_scan` reference using `is_match_symbols(input.chars())`, which consumes all input
without the string matcher's sink detection. Rejected inputs are timed per call,
so future early-exit implementations can be compared without claiming they scanned
the full input. Each case warms up before timing at least 300 ms of repeated operations.
Results are a lightweight baseline, without statistical confidence estimates; compare
repeated runs on the same machine and toolchain with other workloads quiet.

For example, two sequential runs on an AMD Ryzen AI MAX+ PRO 395 under WSL,
with rustc 1.98.1 and the default bench profile, produced these ranges per call.
No builds or tests were run concurrently with these measurements. These are local
observations, not portable performance guarantees or statistical confidence intervals.

| Input | String matcher with sink rejection | Full scan |
| --- | ---: | ---: |
| Valid 9-byte identifier | 71–76 ns | 69–72 ns |
| Invalid 6-byte identifier, first character rejected | 10 ns | 29–31 ns |
| Valid 114,688-byte ASCII identifier | 0.99–1.14 ms | 1.00–1.04 ms |
| Valid 114,688-byte Unicode identifier | 0.54–0.60 ms | 0.55–0.60 ms |
| Invalid 114,689-byte identifier, first character rejected | 10 ns | 0.53 ms |
| Invalid 114,689-byte identifier, last character rejected | 1.02–1.04 ms | 0.97–0.98 ms |
| Valid short float (`12E+99_f64`) | 54–60 ns | 52 ns |
| Invalid short float (`1__0`) | 19 ns | 25–26 ns |

Sink detection costs a few nanoseconds on successful short inputs, and the extra
per-character check can slow long inputs. It pays off when input enters a rejecting
sink early. Both paths still use RangeMapBlaze lookups; the optimization adds no
cached state metadata or per-match allocation. The current string matcher finds one
rejecting sink per call; a minimized DFA has at most one. An unminimized DFA with
multiple sinks can miss an early-exit opportunity, while preserving match results.

## RSB in minimization

During DFA minimization, destinations are grouped into equivalent partitions. RSB's
`transform_values` lets us replace each destination with its partition ID while
automatically merging touching ranges whose new values are equal:

```rust
let transitions = self.transitions[state]
    .range_values()
    .transform_values(|next| {
        block_of[next.id()].expect("reachable state has a block")
    })
    .map(|(range, Owned(block))| (*range.start(), *range.end(), block))
    .collect();
```

This excerpt comes from the internal minimization loop in `src/dfa.rs`, rather than
a standalone program. If `a..=m` and `n..=z` lead to different states in the same
partition, the transformed stream contains a single `a..=z` range. Comparing these
canonical signatures allows equivalent source states to merge, even when their
original range boundaries differed. The operation streams directly into the signature
vector without building an intermediate range map.
