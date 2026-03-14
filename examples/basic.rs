use std::collections::{HashMap, VecDeque};
use std::io;

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

    // range_map_regex::display::display_dfa(
    //     lower_case.start,
    //     &lower_case.transitions,
    //     |index| lower_case.accepting[index],
    //     |state| state.0,
    // )?;

    let upper_case = TinyRegex::new(RangeSetBlaze::from_iter(['A'..='Z']));
    assert!(upper_case.is_match("A"));
    assert!(!upper_case.is_match("a"));
    // range_map_regex::display::display_dfa(
    //     upper_case.start,
    //     &upper_case.transitions,
    //     |index| upper_case.accepting[index],
    //     |state| state.0,
    // )?;

    let letter = lower_case.union(&upper_case);
    assert!(letter.is_match("a"));
    assert!(letter.is_match("Z"));
    assert!(!letter.is_match(""));
    assert!(!letter.is_match("ab"));
    assert!(!letter.is_match("7"));
    assert!(!letter.is_match("é"));
    range_map_regex::display::display_dfa(
        letter.start,
        &letter.transitions,
        |index| letter.accepting[index],
        |state| state.0,
    )?;

    println!("All tests passed!");
    Ok(())
}
