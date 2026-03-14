use std::ops::RangeInclusive;

use indexmap::IndexMap;
use range_set_blaze::{RangeMapBlaze, SortedDisjointMap};

const CHAR_UNIVERSE: RangeInclusive<char> = char::MIN..=char::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateId {
    id: usize,
}

impl StateId {
    pub fn id(self) -> usize {
        self.id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

pub struct Dfa {
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

    fn step(&self, state: StateId, ch: char) -> StateId {
        self.assert_invariants();
        self.transitions[state.id()][ch]
    }

    fn is_accepting_state(&self, state: StateId) -> bool {
        self.assert_invariants();
        self.state_kinds[state.id()] == StateKind::Accepting
    }

    pub fn from_accept_set(range: RangeInclusive<char>) -> Self {
        let mut dfa = Dfa::new(StateKind::Rejecting);
        dfa.assert_invariants();

        // There is a dead state that you can't leave.
        let dead = dfa.new_state(StateKind::Rejecting);

        // There is an accept state, but if you leave it you are dead.
        let accept = dfa.new_state(StateKind::Accepting);
        dfa.set_transitions(accept, RangeMapBlaze::universe_with(&dead));

        // From the start state, you can go to the accept state on the accept range, but otherwise you are dead.
        dfa.set_transitions(
            dfa.start,
            RangeMapBlaze::from_iter([
                (CHAR_UNIVERSE, dead), // send all to dead state
                (range, accept),       // except the accept range
            ]),
        );

        dfa.assert_invariants();

        dfa
    }

    pub fn union(&self, other: &Self) -> Self {
        self.assert_invariants();
        other.assert_invariants();
        // Union product: the combined start state is accepting if either input start state accepts.
        let start_kind = self.state_kinds[self.start.id()].union(other.state_kinds[other.start.id()]);
        let mut dfa = Dfa::new(start_kind);

        let mut pair_to_state: IndexMap<(StateId, StateId), StateId> = IndexMap::new();
        pair_to_state.insert((self.start, other.start), dfa.start);

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

    pub fn minimize(&self) -> Self {
        self.assert_invariants();

        let state_count = self.transitions.len();
        let mut reachable = vec![false; state_count];
        let mut stack = vec![self.start];
        while let Some(state) = stack.pop() {
            if reachable[state.id()] {
                continue;
            }
            reachable[state.id()] = true;
            for (_, next) in self.transitions[state.id()].range_values() {
                if !reachable[next.id()] {
                    stack.push(*next);
                }
            }
        }

        let reachable_states: Vec<usize> = reachable
            .iter()
            .enumerate()
            .filter_map(|(idx, is_reachable)| is_reachable.then_some(idx))
            .collect();

        let mut block_of: Vec<Option<usize>> = vec![None; state_count];
        let has_rejecting = reachable_states
            .iter()
            .any(|&state| self.state_kinds[state] == StateKind::Rejecting);
        for &state in &reachable_states {
            block_of[state] = Some(match self.state_kinds[state] {
                StateKind::Rejecting => 0,
                StateKind::Accepting => {
                    if has_rejecting {
                        1
                    } else {
                        0
                    }
                }
            });
        }

        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        struct Signature {
            state_kind: StateKind,
            transitions: Vec<(char, char, usize)>,
        }

        loop {
            let mut signature_to_block: IndexMap<Signature, usize> = IndexMap::new();
            let mut next_block_of: Vec<Option<usize>> = vec![None; state_count];

            for &state in &reachable_states {
                let transitions = self.transitions[state]
                    .range_values()
                    .map(|(range, next)| {
                        (
                            *range.start(),
                            *range.end(),
                            block_of[next.id()].expect("reachable state has a block"),
                        )
                    })
                    .collect();
                let signature = Signature {
                    state_kind: self.state_kinds[state],
                transitions,
                };
                let block = if let Some(existing) = signature_to_block.get(&signature) {
                    *existing
                } else {
                    let new_block = signature_to_block.len();
                    signature_to_block.insert(signature, new_block);
                    new_block
                };
                next_block_of[state] = Some(block);
            }

            let changed = reachable_states
                .iter()
                .any(|&state| next_block_of[state] != block_of[state]);
            block_of = next_block_of;
            if !changed {
                break;
            }
        }

        let block_count = reachable_states
            .iter()
            .map(|&state| block_of[state].expect("reachable state has a block"))
            .max()
            .map(|max_block| max_block + 1)
            .unwrap_or(0);
        let mut representative: Vec<Option<usize>> = vec![None; block_count];
        for &state in &reachable_states {
            let block = block_of[state].expect("reachable state has a block");
            if representative[block].is_none() {
                representative[block] = Some(state);
            }
        }

        let start_block = block_of[self.start.id()].expect("start state has a block");
        let start_kind = self.state_kinds
            [representative[start_block].expect("start block has a representative")];
        let mut minimized = Dfa::new(start_kind);
        let mut block_to_state: Vec<Option<StateId>> = vec![None; block_count];
        block_to_state[start_block] = Some(minimized.start);
        for block in 0..block_count {
            if block == start_block {
                continue;
            }
            let state_kind = self.state_kinds
                [representative[block].expect("block has a representative")];
            block_to_state[block] = Some(minimized.new_state(state_kind));
        }

        for block in 0..block_count {
            let rep = representative[block].expect("block has a representative");
            let state = block_to_state[block].expect("block has a mapped state");
            let mapped = self.transitions[rep]
                .range_values()
                .map(|(range, next)| {
                    (
                        range,
                        block_to_state
                            [block_of[next.id()].expect("reachable state has a block")]
                        .expect("block has a mapped state"),
                    )
                });
            minimized.set_transitions(state, RangeMapBlaze::from_iter(mapped));
        }

        minimized.assert_invariants();
        minimized
    }

    pub fn is_match(&self, input: &str) -> bool {
        self.assert_invariants();
        let mut state = self.start;
        for ch in input.chars() {
            state = self.step(state, ch);
        }
        self.is_accepting_state(state)
    }

    pub fn start_state(&self) -> StateId {
        self.start
    }

    pub fn state_count(&self) -> usize {
        self.transitions.len()
    }

    pub fn transitions(&self) -> &[RangeMapBlaze<char, StateId>] {
        &self.transitions
    }

    pub fn is_accepting_index(&self, index: usize) -> bool {
        self.state_kinds[index] == StateKind::Accepting
    }
}
