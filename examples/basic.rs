use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use range_set_blaze::{RangeMapBlaze, RangeSetBlaze, SortedDisjointMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StateId(usize);

struct TinyRegex {
    start: StateId,
    accepting: Vec<bool>,
    transitions: Vec<RangeMapBlaze<char, StateId>>,
}

#[derive(Default)]
struct TinyRegexBuilder {
    accepting: Vec<bool>,
    transitions: Vec<RangeMapBlaze<char, StateId>>,
}

impl TinyRegexBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn fresh_state(&mut self, is_accepting: bool) -> StateId {
        let id = StateId(self.transitions.len());
        self.accepting.push(is_accepting);
        self.transitions.push(RangeMapBlaze::new());
        id
    }

    fn set_transitions(&mut self, state: StateId, map: RangeMapBlaze<char, StateId>) {
        self.transitions[state.0] = map;
    }

    fn build(self, start: StateId) -> TinyRegex {
        TinyRegex {
            start,
            accepting: self.accepting,
            transitions: self.transitions,
        }
    }
}

impl TinyRegex {
    fn new(accept_set: RangeSetBlaze<char>) -> Self {
        let mut builder = TinyRegexBuilder::new();
        let start = builder.fresh_state(false);
        let accept = builder.fresh_state(true);
        let dead = builder.fresh_state(false);

        let mut start_map = RangeMapBlaze::from_iter([(char::MIN..=char::MAX, dead)]);
        start_map.extend(accept_set.ranges().map(|range| (range, accept)));

        let sink_map = RangeMapBlaze::from_iter([(char::MIN..=char::MAX, dead)]);

        builder.set_transitions(start, start_map);
        builder.set_transitions(accept, sink_map.clone());
        builder.set_transitions(dead, sink_map);

        builder.build(start)
    }

    fn union(&self, other: &Self) -> Self {
        let mut builder = TinyRegexBuilder::new();
        let mut pair_to_state: HashMap<(StateId, StateId), StateId> = HashMap::new();
        let mut queue: VecDeque<(StateId, StateId)> = VecDeque::new();

        let start_pair = (self.start, other.start);
        let start = builder.fresh_state(self.is_accepting_state(self.start) || other.is_accepting_state(other.start));
        pair_to_state.insert(start_pair, start);
        queue.push_back(start_pair);

        while let Some((left_state, right_state)) = queue.pop_front() {
            let state_id = *pair_to_state
                .get(&(left_state, right_state))
                .expect("queued state is known");

            let transition_pairs =
                merge_transition_maps(&self.transitions[left_state.0], &other.transitions[right_state.0]);

            let mut merged_out = Vec::new();
            for (range, dst_pair) in transition_pairs.range_values() {
                let next = if let Some(existing) = pair_to_state.get(&dst_pair) {
                    *existing
                } else {
                    let accepting = self.is_accepting_state(dst_pair.0) || other.is_accepting_state(dst_pair.1);
                    let new_id = builder.fresh_state(accepting);
                    pair_to_state.insert(*dst_pair, new_id);
                    queue.push_back(*dst_pair);
                    new_id
                };
                merged_out.push((range, next));
            }

            builder.set_transitions(state_id, RangeMapBlaze::from_iter(merged_out));
        }

        builder.build(start)
    }

    // todo0 what is this?
    fn step(&self, state: StateId, ch: char) -> StateId {
        *self.transitions[state.0]
            .get(ch)
            .expect("state transitions cover all chars")
    }

    // todo0 is this efficient?
    fn is_match(&self, input: &str) -> bool {
        let mut state = self.start;
        for ch in input.chars() {
            state = self.step(state, ch);
        }
        self.is_accepting_state(state)
    }

    fn is_accepting_state(&self, state: StateId) -> bool {
        self.accepting[state.0]
    }
}

fn merge_transition_maps(
    left: &RangeMapBlaze<char, StateId>,
    right: &RangeMapBlaze<char, StateId>,
) -> RangeMapBlaze<char, (StateId, StateId)> {
    RangeMapBlaze::from_iter(
        left.range_values()
            // todo0 is this efficient?
            .zip_intersection(right.range_values())
            .map(|(range, (left_value, right_value))| (range, (*left_value, *right_value))),
    )
}

fn display_regex(regex: &TinyRegex) -> io::Result<()> {
    let mut dot = String::from("digraph TinyRegex {\n  rankdir=LR;\n");
    dot.push_str("  __start [shape=point];\n");
    dot.push_str(&format!("  __start -> s{};\n", regex.start.0));

    for state_index in 0..regex.transitions.len() {
        let shape = if regex.is_accepting_state(StateId(state_index)) {
            "doublecircle"
        } else {
            "circle"
        };
        dot.push_str(&format!("  s{state_index} [shape={shape}];\n"));
    }

    for (from_index, transitions) in regex.transitions.iter().enumerate() {
        let mut first_label_by_destination: Vec<Option<char>> = vec![None; regex.transitions.len()];
        for (range, to_state) in transitions.range_values() {
            let start_char = *range.start();
            let slot = &mut first_label_by_destination[to_state.0];
            *slot = Some(match *slot {
                Some(existing) => existing.min(start_char),
                None => start_char,
            });
        }

        for (to_index, maybe_label_char) in first_label_by_destination.into_iter().enumerate() {
            if let Some(label_char) = maybe_label_char {
                let label = escape_dot_label(label_char);
                dot.push_str(&format!(
                    "  s{from_index} -> s{to_index} [label=\"{label}\"];\n"
                ));
            }
        }
    }
    dot.push_str("}\n");

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_millis();
    let base_name = format!("tiny_regex_{stamp}");
    let dot_path: PathBuf = std::env::temp_dir().join(format!("{base_name}.dot"));
    let svg_path: PathBuf = std::env::temp_dir().join(format!("{base_name}.svg"));

    fs::write(&dot_path, dot)?;

    let dot_status = Command::new("dot")
        .arg("-Tsvg")
        .arg(&dot_path)
        .arg("-o")
        .arg(&svg_path)
        .status()
        .map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "failed to run `dot` (install Graphviz). DOT file: {}",
                    dot_path.display()
                ),
            )
        })?;
    if !dot_status.success() {
        return Err(io::Error::other("`dot` failed to render graph"));
    }

    let open_status = Command::new("wslview").arg(&svg_path).status();
    let open_status = match open_status {
        Ok(status) if status.success() => status,
        _ => Command::new("xdg-open")
            .arg(&svg_path)
            .status()
            .map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "failed to run `wslview` or `xdg-open`. Install `wslu` (for WSL) \
or `xdg-utils`. SVG file: {}",
                        svg_path.display()
                    ),
                )
            })?,
    };
    if !open_status.success() {
        return Err(io::Error::other("viewer command failed to open graph"));
    }

    Ok(())
}

fn escape_dot_label(ch: char) -> String {
    match ch {
        '"' => "\\\"".to_owned(),
        '\\' => "\\\\".to_owned(),
        '\0' => "\\\\0".to_owned(),
        '\n' => "\\\\n".to_owned(),
        '\r' => "\\\\r".to_owned(),
        '\t' => "\\\\t".to_owned(),
        _ if ch.is_control() => format!("U+{:04X}", ch as u32),
        _ => ch.to_string(),
    }
}

fn main() {
    if let Err(err) = inner_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn inner_main() -> io::Result<()> {
    let lower_case = TinyRegex::new(RangeSetBlaze::from_iter(['a'..='z']));

    assert!(lower_case.is_match("a"));
    assert!(!lower_case.is_match("A"));

    // display_regex(&lower_case)?;

    let upper_case = TinyRegex::new(RangeSetBlaze::from_iter(['A'..='Z']));
    assert!(upper_case.is_match("A"));
    assert!(!upper_case.is_match("a"));
    // display_regex(&upper_case)?;

    let letter = lower_case.union(&upper_case);
    assert!(letter.is_match("a"));
    assert!(letter.is_match("Z"));
    assert!(!letter.is_match(""));
    assert!(!letter.is_match("ab"));
    assert!(!letter.is_match("7"));
    assert!(!letter.is_match("é"));
    display_regex(&letter)?;

    println!("All tests passed!");
    Ok(())
}
