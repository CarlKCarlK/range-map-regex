use std::io;
use std::ops::RangeInclusive;

use indexmap::IndexMap;
use range_set_blaze::{RangeMapBlaze, SortedDisjointMap};

const CHAR_UNIVERSE: RangeInclusive<char> = char::MIN..=char::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StateId {
    id: usize,
}

impl StateId {
    fn id(self) -> usize {
        self.id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StateKind {
    Accepting,
    Rejecting,
}

impl StateKind {
    fn union(self, other: Self) -> Self {
        if self == StateKind::Accepting || other == StateKind::Accepting {
            StateKind::Accepting
        } else {
            StateKind::Rejecting
        }
    }
}

struct Dfa {
    start: StateId,
    state_kinds: Vec<StateKind>,
    transitions: Vec<RangeMapBlaze<char, StateId>>,
}

impl Dfa {
    fn new(start_kind: StateKind) -> Self {
        let start = StateId { id: 0 };
        let dfa = Self {
            start,
            state_kinds: vec![start_kind],
            transitions: vec![RangeMapBlaze::universe_with(&start)],
        };
        dfa.assert_invariants();
        dfa
    }

    fn new_state(&mut self, state_kind: StateKind) -> StateId {
        self.assert_invariants();
        let id = StateId {
            id: self.transitions.len(),
        };
        self.state_kinds.push(state_kind);
        self.transitions.push(RangeMapBlaze::universe_with(&id));
        assert!(self.transitions[id.id()].is_universal());
        self.assert_invariants();
        id
    }

    fn start(&self) -> StateId {
        self.start
    }

    fn set_transitions(&mut self, state: StateId, map: RangeMapBlaze<char, StateId>) {
        self.assert_invariants();
        assert!(map.is_universal());
        self.transitions[state.id()] = map;
        self.assert_invariants();
    }

    fn assert_invariants(&self) {
        assert_eq!(self.state_kinds.len(), self.transitions.len());
        assert!(self.start.id() < self.transitions.len());
        for transition_map in &self.transitions {
            assert!(transition_map.is_universal());
        }
    }

    fn from_accept_set(range: RangeInclusive<char>) -> Self {
        let mut dfa = Dfa::new(StateKind::Rejecting);
        dfa.assert_invariants();

        // There is a dead state that you can't leave.
        let dead = dfa.new_state(StateKind::Rejecting);

        // There is an accept state, but if you leave it you are dead.
        let accept = dfa.new_state(StateKind::Accepting);
        dfa.set_transitions(accept, RangeMapBlaze::universe_with(&dead));

        // From the start state, you can go to the accept state on the accept range, but otherwise you are dead.
        dfa.set_transitions(
            dfa.start(),
            RangeMapBlaze::from_iter([
                (CHAR_UNIVERSE, dead), // send all to dead state
                (range, accept),       // except the accept range
            ]),
        );

        dfa.assert_invariants();

        dfa
    }

    fn union(&self, other: &Self) -> Self {
        self.assert_invariants();
        other.assert_invariants();
        // Union product: the combined start state is accepting if either input start state accepts.
        let start_kind =
            self.state_kinds[self.start.id()].union(other.state_kinds[other.start.id()]);
        let mut dfa = Dfa::new(start_kind);

        let mut pair_to_state: IndexMap<(StateId, StateId), StateId> = IndexMap::new();
        pair_to_state.insert((self.start, other.start), dfa.start());

        // For each new state that we haven't visited yet....
        let mut cursor = 0;
        while let Some((&(left_state, right_state), &state_id)) = pair_to_state.get_index(cursor) {
            // For each range that transition to same pair ...
            let mut merged_out = Vec::new();
            for (range, (left_next, right_next)) in self.transitions[left_state.id()]
                .range_values()
                .zip_intersection(other.transitions[right_state.id()].range_values())
            {
                let left_next = *left_next;
                let right_next = *right_next;
                let next_pair = (left_next, right_next);

                // If we've seen this pair before, get the id. Otherwise, make a new state for it and remember it.
                let next = if let Some(existing) = pair_to_state.get(&next_pair) {
                    *existing
                } else {
                    let state_kind =
                        self.state_kinds[left_next.id()].union(other.state_kinds[right_next.id()]);
                    let new_id = dfa.new_state(state_kind);
                    pair_to_state.insert(next_pair, new_id);
                    new_id
                };
                merged_out.push((range, next));
            }

            dfa.set_transitions(state_id, RangeMapBlaze::from_iter(merged_out));
            cursor += 1;
        }
        dfa.assert_invariants();

        dfa
    }

    fn step(&self, state: StateId, ch: char) -> StateId {
        self.assert_invariants();
        self.transitions[state.id()][ch]
    }

    fn is_match(&self, input: &str) -> bool {
        self.assert_invariants();
        let mut state = self.start;
        for ch in input.chars() {
            state = self.step(state, ch);
        }
        self.is_accepting_state(state)
    }

    fn is_accepting_state(&self, state: StateId) -> bool {
        self.assert_invariants();
        self.state_kinds[state.id()] == StateKind::Accepting
    }
}

fn main() {
    if let Err(err) = inner_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn inner_main() -> io::Result<()> {
    let lower_case = Dfa::from_accept_set('a'..='z');

    assert!(lower_case.is_match("a"));
    assert!(!lower_case.is_match("A"));

    // range_map_regex::display::display_dfa(
    //     lower_case.start,
    //     &lower_case.transitions,
    //     |index| lower_case.state_kinds[index] == StateKind::Accepting,
    //     |state| state.id,
    // )?;

    let upper_case = Dfa::from_accept_set('A'..='Z');
    assert!(upper_case.is_match("A"));
    assert!(!upper_case.is_match("a"));
    // range_map_regex::display::display_dfa(
    //     upper_case.start,
    //     &upper_case.transitions,
    //     |index| upper_case.state_kinds[index] == StateKind::Accepting,
    //     |state| state.id,
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
        |state| state.id,
    )?;

    println!("All tests passed!");
    Ok(())
}
