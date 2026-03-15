use std::ops::{BitAnd, BitOr, RangeInclusive};

use crate::state_id_set::StateIdSet;
use indexmap::IndexMap;
use range_set_blaze::{Integer, RangeMapBlaze, RangeSetBlaze, SortedDisjointMap};

const CHAR_UNIVERSE: RangeInclusive<char> = char::MIN..=char::MAX;

fn map_values<K, V, W>(
    map: &RangeMapBlaze<K, V>,
    mut f: impl FnMut(&V) -> W,
) -> RangeMapBlaze<K, W>
where
    K: Integer,
    V: Eq + Clone,
    W: Eq + Clone,
{
    RangeMapBlaze::from_iter(map.range_values().map(|(range, value)| (range, f(value))))
}

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

pub struct Dfa<S: Integer = char> {
    start: StateId,
    state_kinds: Vec<StateKind>,
    transitions: Vec<RangeMapBlaze<S, StateId>>,
}

impl<S: Integer + std::hash::Hash> Dfa<S> {
    pub fn empty() -> Self {
        Self::new(StateKind::Rejecting)
    }

    pub fn epsilon() -> Self {
        let mut dfa = Self::new(StateKind::Accepting);
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

    fn set_transitions(&mut self, state: StateId, map: RangeMapBlaze<S, StateId>) {
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

    fn step(&self, state: StateId, symbol: S) -> StateId {
        self.assert_invariants();
        self.transitions[state.id()][symbol]
    }

    fn state_kind(&self, state: StateId) -> StateKind {
        self.assert_invariants();
        self.state_kinds[state.id()]
    }

    fn is_accepting(&self, state: StateId) -> bool {
        self.state_kind(state) == StateKind::Accepting
    }

    pub fn union(&self, other: &Self) -> Self {
        self.assert_invariants();
        other.assert_invariants();
        // Union product: the combined start state is accepting if either input start state accepts.
        let start_kind = self.state_kind(self.start) | other.state_kind(other.start);
        let mut dfa = Self::new(start_kind);

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
        let mut dfa = Self::new(start_kind);

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
        Self::epsilon().union(self)
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
        let mut dfa = Self::new(start_kind);

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
        let mut dfa = Self::new(StateKind::Accepting);
        // `StateKind` is intentionally part of the key.
        // The same active subset may occur with different boundary acceptance.
        // Example: `Dfa::empty().star()` has `{start}` at:
        // - start boundary (accepting, for zero repetitions), and
        // - after consuming any symbol (rejecting).
        // If keyed only by subset, those two states would merge incorrectly.
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

        loop {
            let mut signature_to_block: IndexMap<(StateKind, Vec<(S, S, usize)>), usize> =
                IndexMap::new();
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
                let signature = (self.state_kinds[state], transitions);
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
        let mut minimized = Self::new(start_kind);
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
            let mapped = map_values(&self.transitions[rep], |next| {
                block_to_state[block_of[next.id()].expect("reachable state has a block")]
                    .expect("block has a mapped state")
            });
            minimized.set_transitions(state, mapped);
        }

        minimized.assert_invariants();
        minimized
    }

    pub fn start_state(&self) -> StateId {
        self.start
    }

    pub fn state_count(&self) -> usize {
        self.transitions.len()
    }

    pub fn is_match_symbols(&self, input: impl IntoIterator<Item = S>) -> bool {
        self.assert_invariants();
        let mut state = self.start;
        for symbol in input {
            state = self.step(state, symbol);
        }
        self.is_accepting(state)
    }

    pub fn transitions(&self) -> &[RangeMapBlaze<S, StateId>] {
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
        right: &Dfa<S>,
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
    ) -> RangeMapBlaze<S, StateIdSet> {
        let mut iter = source_indices.iter();

        // If empty, the transition is to the empty set on all symbols.
        let Some(first) = iter.next() else {
            return RangeMapBlaze::universe_with(&StateIdSet::new());
        };

        // For the 1st source, the transition is to the singleton set of its target on each symbol.
        let mut acc = map_values(&self.transitions[first.id()], |next| StateIdSet::from_state(*next));

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
}

impl Dfa<char> {
    pub fn from_char_set(accept_set: RangeSetBlaze<char>) -> Self {
        let mut dfa = Self::new(StateKind::Rejecting);
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

    pub fn from_char(ch: char) -> Self {
        Self::from_char_range(ch..=ch)
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
        s.chars()
            .fold(Self::epsilon(), |dfa, ch| dfa.concat(&Self::from_char(ch)))
    }

    pub fn is_match(&self, input: &str) -> bool {
        self.assert_invariants();
        let mut state = self.start;
        for ch in input.chars() {
            state = self.step(state, ch);
        }
        self.is_accepting(state)
    }

    pub fn to_utf8_dfa(&self) -> Dfa<u8> {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        enum DecodeState {
            Ready,
            Pending {
                code_point_prefix: u32,
                remaining: u8,
                min_code_point: u32,
                next_cont_min: u8,
                next_cont_max: u8,
            },
            Reject,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        struct ProductState {
            char_state: StateId,
            decode_state: DecodeState,
        }

        #[derive(Debug, Clone, Copy)]
        enum DecodeStep {
            Pending(DecodeState),
            Complete(char),
            Reject,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum ByteClass {
            Ascii,
            ContLow,
            ContMid,
            ContHigh,
            Lead2,
            Lead3E0,
            Lead3E1Ec,
            Lead3Ed,
            Lead3EeEf,
            Lead4F0,
            Lead4F1F3,
            Lead4F4,
            Invalid,
        }

        fn decode_step(decode_state: DecodeState, byte: u8) -> DecodeStep {
            match decode_state {
                DecodeState::Reject => DecodeStep::Reject,
                DecodeState::Ready => match byte {
                    0x00..=0x7F => DecodeStep::Complete(byte as char),
                    0xC2..=0xDF => DecodeStep::Pending(DecodeState::Pending {
                        code_point_prefix: (byte & 0x1F) as u32,
                        remaining: 1,
                        min_code_point: 0x80,
                        next_cont_min: 0x80,
                        next_cont_max: 0xBF,
                    }),
                    0xE0 => DecodeStep::Pending(DecodeState::Pending {
                        code_point_prefix: (byte & 0x0F) as u32,
                        remaining: 2,
                        min_code_point: 0x800,
                        next_cont_min: 0xA0,
                        next_cont_max: 0xBF,
                    }),
                    0xE1..=0xEC | 0xEE..=0xEF => DecodeStep::Pending(DecodeState::Pending {
                        code_point_prefix: (byte & 0x0F) as u32,
                        remaining: 2,
                        min_code_point: 0x800,
                        next_cont_min: 0x80,
                        next_cont_max: 0xBF,
                    }),
                    0xED => DecodeStep::Pending(DecodeState::Pending {
                        code_point_prefix: (byte & 0x0F) as u32,
                        remaining: 2,
                        min_code_point: 0x800,
                        next_cont_min: 0x80,
                        next_cont_max: 0x9F,
                    }),
                    0xF0 => DecodeStep::Pending(DecodeState::Pending {
                        code_point_prefix: (byte & 0x07) as u32,
                        remaining: 3,
                        min_code_point: 0x10000,
                        next_cont_min: 0x90,
                        next_cont_max: 0xBF,
                    }),
                    0xF1..=0xF3 => DecodeStep::Pending(DecodeState::Pending {
                        code_point_prefix: (byte & 0x07) as u32,
                        remaining: 3,
                        min_code_point: 0x10000,
                        next_cont_min: 0x80,
                        next_cont_max: 0xBF,
                    }),
                    0xF4 => DecodeStep::Pending(DecodeState::Pending {
                        code_point_prefix: (byte & 0x07) as u32,
                        remaining: 3,
                        min_code_point: 0x10000,
                        next_cont_min: 0x80,
                        next_cont_max: 0x8F,
                    }),
                    _ => DecodeStep::Reject,
                },
                DecodeState::Pending {
                    code_point_prefix,
                    remaining,
                    min_code_point,
                    next_cont_min,
                    next_cont_max,
                } => {
                    if byte < next_cont_min || byte > next_cont_max {
                        return DecodeStep::Reject;
                    }
                    let next_prefix = (code_point_prefix << 6) | ((byte & 0x3F) as u32);
                    if remaining > 1 {
                        DecodeStep::Pending(DecodeState::Pending {
                            code_point_prefix: next_prefix,
                            remaining: remaining - 1,
                            min_code_point,
                            next_cont_min: 0x80,
                            next_cont_max: 0xBF,
                        })
                    } else if (next_prefix < min_code_point)
                        || (next_prefix > 0x10FFFF)
                        || (0xD800..=0xDFFF).contains(&next_prefix)
                    {
                        DecodeStep::Reject
                    } else if let Some(ch) = char::from_u32(next_prefix) {
                        DecodeStep::Complete(ch)
                    } else {
                        DecodeStep::Reject
                    }
                }
            }
        }

        let start_product = ProductState {
            char_state: self.start,
            decode_state: DecodeState::Ready,
        };
        let start_kind = self.state_kind(self.start);
        let mut utf8_dfa = Dfa::<u8>::new(start_kind);
        let mut product_to_state: IndexMap<ProductState, StateId> = IndexMap::new();
        let reject_sink_char_state = self
            .transitions
            .iter()
            .enumerate()
            .find_map(|(state_index, transition_map)| {
                let candidate = StateId::from_id(state_index);
                (self.state_kind(candidate) == StateKind::Rejecting
                    && transition_map.range_values().all(|(_, next)| *next == candidate))
                .then_some(candidate)
            });
        let canonical_product = |mut product_state: ProductState| {
            if let Some(reject_sink) = reject_sink_char_state {
                if product_state.char_state == reject_sink
                    || product_state.decode_state == DecodeState::Reject
                {
                    product_state.char_state = reject_sink;
                    product_state.decode_state = DecodeState::Ready;
                }
            }
            product_state
        };
        let lead_byte_codepoint_range = |lead_byte: u8| -> Option<(u32, u32)> {
            match lead_byte {
                0xC2..=0xDF => {
                    let prefix = (lead_byte & 0x1F) as u32;
                    let min = prefix << 6;
                    Some((min, min | 0x3F))
                }
                0xE0 => Some((0x800, 0xFFF)),
                0xE1..=0xEC => {
                    let prefix = (lead_byte & 0x0F) as u32;
                    let min = prefix << 12;
                    Some((min, min | 0xFFF))
                }
                0xED => Some((0xD000, 0xD7FF)),
                0xEE..=0xEF => {
                    let prefix = (lead_byte & 0x0F) as u32;
                    let min = prefix << 12;
                    Some((min, min | 0xFFF))
                }
                0xF0 => Some((0x10000, 0x3FFFF)),
                0xF1..=0xF3 => {
                    let prefix = (lead_byte & 0x07) as u32;
                    let min = prefix << 18;
                    Some((min, min | 0x3FFFF))
                }
                0xF4 => Some((0x100000, 0x10FFFF)),
                _ => None,
            }
        };
        let can_transition_on_lead_byte = |char_state: StateId, lead_byte: u8| -> bool {
            let Some(reject_sink) = reject_sink_char_state else {
                return true;
            };
            let Some((codepoint_start, codepoint_end)) = lead_byte_codepoint_range(lead_byte) else {
                return false;
            };
            self.transitions[char_state.id()]
                .range_values()
                .filter(|(_, next_state)| **next_state != reject_sink)
                .any(|(char_range, _)| {
                    let range_start = *char_range.start() as u32;
                    let range_end = *char_range.end() as u32;
                    !(range_end < codepoint_start || codepoint_end < range_start)
                })
        };
        product_to_state.insert(canonical_product(start_product), utf8_dfa.start);
        let byte_classes = RangeMapBlaze::from_iter([
            (0x00u8..=0x7F, ByteClass::Ascii),
            (0x80u8..=0x8F, ByteClass::ContLow),
            (0x90u8..=0x9F, ByteClass::ContMid),
            (0xA0u8..=0xBF, ByteClass::ContHigh),
            (0xC0u8..=0xC1, ByteClass::Invalid),
            (0xC2u8..=0xDF, ByteClass::Lead2),
            (0xE0u8..=0xE0, ByteClass::Lead3E0),
            (0xE1u8..=0xEC, ByteClass::Lead3E1Ec),
            (0xEDu8..=0xED, ByteClass::Lead3Ed),
            (0xEEu8..=0xEF, ByteClass::Lead3EeEf),
            (0xF0u8..=0xF0, ByteClass::Lead4F0),
            (0xF1u8..=0xF3, ByteClass::Lead4F1F3),
            (0xF4u8..=0xF4, ByteClass::Lead4F4),
            (0xF5u8..=0xFF, ByteClass::Invalid),
        ]);

        let mut cursor = 0;
        while let Some((&product_state, &state_id)) = product_to_state.get_index(cursor) {
            let mut byte_transitions: Vec<(RangeInclusive<u8>, StateId)> = Vec::new();
            let mut push_transition = |range: RangeInclusive<u8>, target_state: StateId| {
                if let Some((previous_range, previous_target)) = byte_transitions.last_mut()
                    && *previous_target == target_state
                    && *range.start() == previous_range.end().saturating_add(1)
                {
                    *previous_range = *previous_range.start()..=*range.end();
                    return;
                }
                byte_transitions.push((range, target_state));
            };
            let reject_product = canonical_product(ProductState {
                char_state: self.start,
                decode_state: DecodeState::Reject,
            });
            let reject_target_state = if let Some(existing) = product_to_state.get(&reject_product) {
                *existing
            } else {
                let new_state = utf8_dfa.new_state(StateKind::Rejecting);
                product_to_state.insert(reject_product, new_state);
                new_state
            };

            for (byte_range, byte_class) in byte_classes.range_values() {
                let product_state = canonical_product(product_state);
                match (product_state.decode_state, byte_class) {
                    (DecodeState::Reject, _) => {
                        push_transition(byte_range.clone(), reject_target_state);
                    }
                    (DecodeState::Ready, ByteClass::Ascii) => {
                        // For ASCII, UTF-8 decoding is identity and we can reuse the char transition partition.
                        for (char_range, next_char_state) in
                            self.transitions[product_state.char_state.id()].range_values()
                        {
                            let start = (*char_range.start() as u32).max(0x00);
                            let end = (*char_range.end() as u32).min(0x7F);
                            if start > end {
                                continue;
                            }
                            let next_product = ProductState {
                                char_state: *next_char_state,
                                decode_state: DecodeState::Ready,
                            };
                            let target_state =
                                if let Some(existing) = product_to_state.get(&next_product) {
                                    *existing
                                } else {
                                    let state_kind = if self.is_accepting(next_product.char_state) {
                                        StateKind::Accepting
                                    } else {
                                        StateKind::Rejecting
                                    };
                                    let new_state = utf8_dfa.new_state(state_kind);
                                    product_to_state.insert(next_product, new_state);
                                    new_state
                                };
                            push_transition((start as u8)..=(end as u8), target_state);
                        }
                    }
                    (
                        DecodeState::Ready,
                        ByteClass::Lead2
                        | ByteClass::Lead3E0
                        | ByteClass::Lead3E1Ec
                        | ByteClass::Lead3Ed
                        | ByteClass::Lead3EeEf
                        | ByteClass::Lead4F0
                        | ByteClass::Lead4F1F3
                        | ByteClass::Lead4F4,
                    ) => {
                        for byte in *byte_range.start()..=*byte_range.end() {
                            if !can_transition_on_lead_byte(product_state.char_state, byte) {
                                push_transition(byte..=byte, reject_target_state);
                                continue;
                            }
                            let next_product = canonical_product(match decode_step(product_state.decode_state, byte) {
                                DecodeStep::Pending(next_decode_state) => ProductState {
                                    char_state: product_state.char_state,
                                    decode_state: next_decode_state,
                                },
                                DecodeStep::Complete(ch) => ProductState {
                                    char_state: self.step(product_state.char_state, ch),
                                    decode_state: DecodeState::Ready,
                                },
                                DecodeStep::Reject => ProductState {
                                    char_state: self.start,
                                    decode_state: DecodeState::Reject,
                                },
                            });
                            let target_state =
                                if let Some(existing) = product_to_state.get(&next_product) {
                                    *existing
                                } else {
                                    let state_kind = if next_product.decode_state == DecodeState::Ready
                                        && self.is_accepting(next_product.char_state)
                                    {
                                        StateKind::Accepting
                                    } else {
                                        StateKind::Rejecting
                                    };
                                    let new_state = utf8_dfa.new_state(state_kind);
                                    product_to_state.insert(next_product, new_state);
                                    new_state
                                };
                            push_transition(byte..=byte, target_state);
                        }
                    }
                    (
                        DecodeState::Ready,
                        ByteClass::ContLow | ByteClass::ContMid | ByteClass::ContHigh | ByteClass::Invalid,
                    ) => {
                        push_transition(byte_range.clone(), reject_target_state);
                    }
                    (
                        DecodeState::Pending { .. },
                        ByteClass::ContLow | ByteClass::ContMid | ByteClass::ContHigh,
                    ) => {
                        for byte in *byte_range.start()..=*byte_range.end() {
                            let next_product = canonical_product(match decode_step(product_state.decode_state, byte) {
                                DecodeStep::Pending(next_decode_state) => ProductState {
                                    char_state: product_state.char_state,
                                    decode_state: next_decode_state,
                                },
                                DecodeStep::Complete(ch) => ProductState {
                                    char_state: self.step(product_state.char_state, ch),
                                    decode_state: DecodeState::Ready,
                                },
                                DecodeStep::Reject => ProductState {
                                    char_state: self.start,
                                    decode_state: DecodeState::Reject,
                                },
                            });
                            let target_state =
                                if let Some(existing) = product_to_state.get(&next_product) {
                                    *existing
                                } else {
                                    let state_kind = if next_product.decode_state == DecodeState::Ready
                                        && self.is_accepting(next_product.char_state)
                                    {
                                        StateKind::Accepting
                                    } else {
                                        StateKind::Rejecting
                                    };
                                    let new_state = utf8_dfa.new_state(state_kind);
                                    product_to_state.insert(next_product, new_state);
                                    new_state
                                };
                            push_transition(byte..=byte, target_state);
                        }
                    }
                    (DecodeState::Pending { .. }, _) => {
                        push_transition(byte_range.clone(), reject_target_state);
                    }
                }
            }

            utf8_dfa.set_transitions(state_id, RangeMapBlaze::from_iter(byte_transitions));
            cursor += 1;
        }

        utf8_dfa.assert_invariants();
        utf8_dfa
    }

    fn char_set_from_predicate(mut predicate: impl FnMut(char) -> bool) -> RangeSetBlaze<char> {
        RangeSetBlaze::from_iter(CHAR_UNIVERSE.clone().filter(|&ch| predicate(ch)))
    }
}

impl Dfa<u8> {
    pub fn is_match_bytes(&self, input: &[u8]) -> bool {
        self.is_match_symbols(input.iter().copied())
    }
}
