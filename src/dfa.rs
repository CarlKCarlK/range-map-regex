use std::ops::{BitAnd, BitOr, RangeInclusive};

use crate::state_id_set::StateIdSet;
use indexmap::IndexMap;
use range_set_blaze::{RangeMapBlaze, RangeSetBlaze, SortedDisjointMap};

const CHAR_UNIVERSE: RangeInclusive<char> = char::MIN..=char::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateId {
    id: usize,
}

impl StateId {
    pub fn id(self) -> usize {
        self.id
    }

    pub(crate) fn from_id(id: usize) -> Self {
        Self { id }
    }
}

impl Ord for StateId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for StateId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum StateKind {
    Accepting,
    Rejecting,
}

impl StateKind {
    fn complement(self) -> Self {
        match self {
            StateKind::Accepting => StateKind::Rejecting,
            StateKind::Rejecting => StateKind::Accepting,
        }
    }
}

impl BitOr for StateKind {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        if self == StateKind::Accepting || rhs == StateKind::Accepting {
            StateKind::Accepting
        } else {
            StateKind::Rejecting
        }
    }
}

impl BitAnd for StateKind {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        if self == StateKind::Accepting && rhs == StateKind::Accepting {
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
    pub fn empty() -> Self {
        Dfa::new(StateKind::Rejecting)
    }

    pub fn epsilon() -> Self {
        let mut dfa = Dfa::new(StateKind::Accepting);
        let dead = dfa.new_state(StateKind::Rejecting);
        dfa.set_transitions(dfa.start, RangeMapBlaze::universe_with(&dead));
        dfa.assert_invariants();
        dfa
    }

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

    fn state_kind(&self, state: StateId) -> StateKind {
        self.assert_invariants();
        self.state_kinds[state.id()]
    }

    fn is_accepting(&self, state: StateId) -> bool {
        self.state_kind(state) == StateKind::Accepting
    }

    pub fn from_char_set(accept_set: RangeSetBlaze<char>) -> Self {
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
            RangeMapBlaze::from_iter(
                std::iter::once((CHAR_UNIVERSE, dead)) // send all to dead state
                    .chain(accept_set.ranges().map(|range| (range, accept))), // except the accept range
            ),
        );

        dfa.assert_invariants();

        dfa
    }

    pub fn from_char_range(range: RangeInclusive<char>) -> Self {
        Self::from_char_set(RangeSetBlaze::from_iter([range]))
    }

    pub fn from_chars_where(predicate: impl FnMut(char) -> bool) -> Self {
        Self::from_char_set(Self::char_set_from_predicate(predicate))
    }

    pub fn xid_start() -> Self {
        Self::from_chars_where(unicode_ident::is_xid_start)
    }

    pub fn xid_continue() -> Self {
        Self::from_chars_where(unicode_ident::is_xid_continue)
    }

    pub fn string(s: &str) -> Self {
        s.chars().fold(Dfa::epsilon(), |dfa, ch| {
            dfa.concat(&Dfa::from_char_range(ch..=ch))
        })
    }

    pub fn union(&self, other: &Self) -> Self {
        self.assert_invariants();
        other.assert_invariants();
        // Union product: the combined start state is accepting if either input start state accepts.
        let start_kind = self.state_kind(self.start) | other.state_kind(other.start);
        let mut dfa = Dfa::new(start_kind);

        let mut pair_to_state: IndexMap<(StateId, StateId), StateId> = IndexMap::new();
        pair_to_state.insert((self.start, other.start), dfa.start);

        // For each new state that we haven't visited yet....
        let mut cursor = 0;
        while let Some((&(left_state, right_state), &state_id)) = pair_to_state.get_index(cursor) {
            let merged_out = RangeMapBlaze::from_iter(
                self.transitions[left_state.id()]
                    .range_values()
                    .inner_join(other.transitions[right_state.id()].range_values())
                    .map(|(range, (left_next, right_next))| {
                        let next_pair = (*left_next, *right_next);
                        let next = if let Some(existing) = pair_to_state.get(&next_pair) {
                            *existing
                        } else {
                            let state_kind =
                                self.state_kind(next_pair.0) | other.state_kind(next_pair.1);
                            let new_id = dfa.new_state(state_kind);
                            pair_to_state.insert(next_pair, new_id);
                            new_id
                        };
                        (range, next)
                    }),
            );

            dfa.set_transitions(state_id, merged_out);
            cursor += 1;
        }
        dfa.assert_invariants();

        dfa
    }

    pub fn intersection(&self, other: &Self) -> Self {
        self.assert_invariants();
        other.assert_invariants();
        let start_kind = self.state_kind(self.start) & other.state_kind(other.start);
        let mut dfa = Dfa::new(start_kind);

        let mut pair_to_state: IndexMap<(StateId, StateId), StateId> = IndexMap::new();
        pair_to_state.insert((self.start, other.start), dfa.start);

        let mut cursor = 0;
        while let Some((&(left_state, right_state), &state_id)) = pair_to_state.get_index(cursor) {
            let merged_out = RangeMapBlaze::from_iter(
                self.transitions[left_state.id()]
                    .range_values()
                    .inner_join(other.transitions[right_state.id()].range_values())
                    .map(|(range, (left_next, right_next))| {
                        let next_pair = (*left_next, *right_next);
                        let next = if let Some(existing) = pair_to_state.get(&next_pair) {
                            *existing
                        } else {
                            let state_kind =
                                self.state_kind(next_pair.0) & other.state_kind(next_pair.1);
                            let new_id = dfa.new_state(state_kind);
                            pair_to_state.insert(next_pair, new_id);
                            new_id
                        };
                        (range, next)
                    }),
            );
            dfa.set_transitions(state_id, merged_out);
            cursor += 1;
        }

        dfa.assert_invariants();
        dfa
    }

    pub fn complement(&self) -> Self {
        self.assert_invariants();
        let dfa = Dfa {
            start: self.start,
            state_kinds: self
                .state_kinds
                .iter()
                .copied()
                .map(StateKind::complement)
                .collect(),
            transitions: self.transitions.clone(),
        };
        dfa.assert_invariants();
        dfa
    }

    pub fn optional(&self) -> Self {
        Dfa::epsilon().union(self)
    }

    pub fn plus(&self) -> Self {
        self.concat(&self.star())
    }

    // todo00 study this
    pub fn concat(&self, right: &Self) -> Self {
        self.assert_invariants();
        right.assert_invariants();

        // Active right states at the current boundary between left and right.
        let mut right_active = StateIdSet::new();
        // If left accepts empty at start, right may start immediately.
        if self.is_accepting(self.start) {
            right_active.insert(right.start);
        }
        let start_kind = self.concat_state_kind(self.start, right, &right_active);

        // Build the concatenated DFA lazily from discovered product keys.
        let mut dfa = Dfa::new(start_kind);

        // Key: (left state, active-right-state subset) -> concatenated DFA state.
        let mut key_to_state = IndexMap::new();
        key_to_state.insert((self.start, right_active), dfa.start);

        let mut cursor = 0;
        while let Some(((left_state, right_active), &state_id)) = key_to_state.get_index(cursor) {
            let left_state = *left_state;
            let mut right_sources = right_active.clone();
            if self.is_accepting(left_state) {
                right_sources.insert(right.start);
            }

            let right_next_map = right.subset_transition_map(&right_sources);
            let merged_out = RangeMapBlaze::from_iter(
                self.transitions[left_state.id()]
                    .range_values()
                    .inner_join(right_next_map.range_values())
                    .map(|(range, (left_next, right_next_active))| {
                        let next_right_active = right_next_active.clone();
                        let next_key = (*left_next, next_right_active.clone());
                        let next = if let Some(existing) = key_to_state.get(&next_key) {
                            *existing
                        } else {
                            let state_kind =
                                self.concat_state_kind(next_key.0, right, &next_right_active);
                            let new_id = dfa.new_state(state_kind);
                            key_to_state.insert(next_key, new_id);
                            new_id
                        };
                        (range, next)
                    }),
            );

            dfa.set_transitions(state_id, merged_out);
            cursor += 1;
        }

        dfa.assert_invariants();
        dfa
    }

    pub fn star(&self) -> Self {
        self.assert_invariants();

        let start_active = StateIdSet::from_state(self.start);
        let mut dfa = Dfa::new(StateKind::Accepting);
        let mut key_to_state: IndexMap<(StateIdSet, StateKind), StateId> = IndexMap::new();
        key_to_state.insert((start_active, StateKind::Accepting), dfa.start);

        let mut cursor = 0;
        while let Some(((active, _boundary_kind), &state_id)) = key_to_state.get_index(cursor) {
            let active = active.clone();
            let next_map = self.subset_transition_map(&active);
            let merged_out = RangeMapBlaze::from_iter(next_map.range_values().map(
                |(range, next_active)| {
                    let mut next_active = next_active.clone();
                    let boundary_kind = self.any_accepting(&next_active);
                    if boundary_kind == StateKind::Accepting {
                        // Once we can end one repetition, the next repetition may start immediately.
                        // Include the original start state in the active subset for following input.
                        // todo0 this subset expansion may be optimized.
                        next_active.insert(self.start);
                    }

                    let next_key = (next_active.clone(), boundary_kind);
                    let next = if let Some(existing) = key_to_state.get(&next_key) {
                        *existing
                    } else {
                        let new_id = dfa.new_state(boundary_kind);
                        key_to_state.insert(next_key, new_id);
                        new_id
                    };
                    (range, next)
                },
            ));

            dfa.set_transitions(state_id, merged_out);
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
            let state_kind =
                self.state_kinds[representative[block].expect("block has a representative")];
            block_to_state[block] = Some(minimized.new_state(state_kind));
        }

        for block in 0..block_count {
            let rep = representative[block].expect("block has a representative");
            let state = block_to_state[block].expect("block has a mapped state");
            let mapped = self.transitions[rep].range_values().map(|(range, next)| {
                (
                    range,
                    block_to_state[block_of[next.id()].expect("reachable state has a block")]
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
        self.is_accepting(state)
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
        self.is_accepting(StateId { id: index })
    }

    fn any_accepting(&self, active: &StateIdSet) -> StateKind {
        active
            .iter()
            .fold(StateKind::Rejecting, |acc, state_index| {
                acc | self.state_kinds[state_index.id()]
            })
    }

    fn concat_state_kind(
        &self,
        left_state: StateId,
        right: &Dfa,
        right_active: &StateIdSet,
    ) -> StateKind {
        right.any_accepting(right_active)
            | if self.is_accepting(left_state) && right.is_accepting(right.start) {
                StateKind::Accepting
            } else {
                StateKind::Rejecting
            }
    }

    fn subset_transition_map(
        &self,
        source_indices: &StateIdSet,
    ) -> RangeMapBlaze<char, StateIdSet> {
        let mut iter = source_indices.iter();

        // If empty, the transition is to the empty set on all characters.
        let Some(first) = iter.next() else {
            return RangeMapBlaze::universe_with(&StateIdSet::new());
        };

        // For the 1st source, the transition is to the singleton set of its target on each character.
        let mut acc = RangeMapBlaze::from_iter(
            self.transitions[first.id()]
                .range_values()
                .map(|(range, next)| (range, StateIdSet::from_state(*next))),
        );

        // For each subsequent source, intersect the current map with the singleton map of its targets, and union the targets into the resulting sets.
        for source in iter {
            acc = RangeMapBlaze::from_iter(
                acc.range_values()
                    .inner_join(self.transitions[source.id()].range_values())
                    .map(|(range, (next_set, next))| (range, next_set.with_inserted(*next))),
            );
        }

        acc
    }

    fn char_set_from_predicate(mut predicate: impl FnMut(char) -> bool) -> RangeSetBlaze<char> {
        RangeSetBlaze::from_iter(CHAR_UNIVERSE.clone().filter(|&ch| predicate(ch)))
    }
}
