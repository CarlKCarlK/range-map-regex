use std::collections::{HashMap, VecDeque};
use std::io;

use range_set_blaze::{RangeMapBlaze, RangeSetBlaze, SortedDisjointMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StateId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StateKind {
    Accepting,
    Rejecting,
}

struct Dfa {
    start: StateId,
    state_kinds: Vec<StateKind>,
    transitions: Vec<RangeMapBlaze<char, StateId>>,
}

impl Dfa {
    fn new(start_kind: StateKind) -> Self {
        let start = StateId(0);
        Self {
            start,
            state_kinds: vec![start_kind],
            transitions: vec![RangeMapBlaze::from_iter([(char::MIN..=char::MAX, start)])],
        }
    }

    fn new_state(&mut self, state_kind: StateKind) -> StateId {
        let id = StateId(self.transitions.len());
        self.state_kinds.push(state_kind);
        self.transitions
            .push(RangeMapBlaze::from_iter([(char::MIN..=char::MAX, id)]));
        id
    }

    fn start(&self) -> StateId {
        self.start
    }

    fn set_transitions(&mut self, state: StateId, map: RangeMapBlaze<char, StateId>) {
        let mut total_map = RangeMapBlaze::from_iter([(char::MIN..=char::MAX, state)]);
        total_map.extend(map.range_values().map(|(range, dst)| (range, *dst)));
        self.transitions[state.0] = total_map;
    }

}

impl Dfa {
    fn from_accept_set(accept_set: RangeSetBlaze<char>) -> Self {
        let mut dfa = Dfa::new(StateKind::Rejecting);
        let start = dfa.start();
        let accept = dfa.new_state(StateKind::Accepting);
        let dead = dfa.new_state(StateKind::Rejecting);

        let mut start_map = RangeMapBlaze::from_iter([(char::MIN..=char::MAX, dead)]);
        start_map.extend(accept_set.ranges().map(|range| (range, accept)));

        let sink_map = RangeMapBlaze::from_iter([(char::MIN..=char::MAX, dead)]);

        dfa.set_transitions(start, start_map);
        dfa.set_transitions(accept, sink_map.clone());
        dfa.set_transitions(dead, sink_map);

        dfa
    }

    fn union(&self, other: &Self) -> Self {
        let start_kind = if self.is_accepting_state(self.start) || other.is_accepting_state(other.start) {
            StateKind::Accepting
        } else {
            StateKind::Rejecting
        };
        let mut dfa = Dfa::new(start_kind);
        let mut pair_to_state: HashMap<(StateId, StateId), StateId> = HashMap::new();
        let mut queue: VecDeque<(StateId, StateId)> = VecDeque::new();

        let start_pair = (self.start, other.start);
        let start = dfa.start();
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
                    let state_kind =
                        if self.is_accepting_state(dst_pair.0) || other.is_accepting_state(dst_pair.1) {
                            StateKind::Accepting
                        } else {
                            StateKind::Rejecting
                        };
                    let new_id = dfa.new_state(state_kind);
                    pair_to_state.insert(*dst_pair, new_id);
                    queue.push_back(*dst_pair);
                    new_id
                };
                merged_out.push((range, next));
            }

            dfa.set_transitions(state_id, RangeMapBlaze::from_iter(merged_out));
        }

        dfa
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
        self.state_kinds[state.0] == StateKind::Accepting
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
    let lower_case = Dfa::from_accept_set(RangeSetBlaze::from_iter(['a'..='z']));

    assert!(lower_case.is_match("a"));
    assert!(!lower_case.is_match("A"));

    // range_map_regex::display::display_dfa(
    //     lower_case.start,
    //     &lower_case.transitions,
    //     |index| lower_case.state_kinds[index] == StateKind::Accepting,
    //     |state| state.0,
    // )?;

    let upper_case = Dfa::from_accept_set(RangeSetBlaze::from_iter(['A'..='Z']));
    assert!(upper_case.is_match("A"));
    assert!(!upper_case.is_match("a"));
    // range_map_regex::display::display_dfa(
    //     upper_case.start,
    //     &upper_case.transitions,
    //     |index| upper_case.state_kinds[index] == StateKind::Accepting,
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
        |index| letter.state_kinds[index] == StateKind::Accepting,
        |state| state.0,
    )?;

    println!("All tests passed!");
    Ok(())
}
