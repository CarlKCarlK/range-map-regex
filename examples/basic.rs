use std::collections::{HashMap, VecDeque};

use range_set_blaze::{RangeMapBlaze, RangeSetBlaze};

type StateId = usize;

struct TinyRegex {
    start: StateId,
    accepting: Vec<bool>,
    transitions: Vec<RangeMapBlaze<char, StateId>>,
}

impl TinyRegex {
    fn new(accept_set: RangeSetBlaze<char>) -> Self {
        let mut start = RangeMapBlaze::from_iter([(char::MIN..=char::MAX, 2_usize)]);
        start.extend(accept_set.ranges().map(|range| (range, 1_usize)));
        let accept = RangeMapBlaze::from_iter([(char::MIN..=char::MAX, 2_usize)]);
        let dead = RangeMapBlaze::from_iter([(char::MIN..=char::MAX, 2_usize)]);

        Self {
            start: 0,
            accepting: vec![false, true, false],
            transitions: vec![start, accept, dead],
        }
    }

    fn union(&self, other: &Self) -> Self {
        let mut pair_to_state: HashMap<(StateId, StateId), StateId> = HashMap::new();
        let mut queue: VecDeque<(StateId, StateId)> = VecDeque::new();
        pair_to_state.insert((self.start, other.start), 0);
        queue.push_back((self.start, other.start));

        let mut accepting = Vec::new();
        let mut transitions = Vec::new();

        while let Some((left_state, right_state)) = queue.pop_front() {
            let state_id = *pair_to_state
                .get(&(left_state, right_state))
                .expect("queued state is known");
            assert_eq!(state_id, accepting.len());

            accepting.push(self.accepting[left_state] || other.accepting[right_state]);

            let mut merged_out = Vec::new();
            let transition_pairs =
                merge_transition_maps(&self.transitions[left_state], &other.transitions[right_state]);
            for (range, dst_pair) in transition_pairs.range_values() {
                let next = if let Some(existing) = pair_to_state.get(&dst_pair) {
                    *existing
                } else {
                    let new_id = pair_to_state.len();
                    pair_to_state.insert(*dst_pair, new_id);
                    queue.push_back(*dst_pair);
                    new_id
                };
                merged_out.push((range, next));
            }

            transitions.push(RangeMapBlaze::from_iter(merged_out));
        }

        Self {
            start: 0,
            accepting,
            transitions,
        }
    }

    fn step(&self, state: StateId, ch: char) -> StateId {
        *self.transitions[state]
            .get(ch)
            .expect("state transitions cover all chars")
    }

    fn is_match(&self, input: &str) -> bool {
        let mut state = self.start;
        for ch in input.chars() {
            state = self.step(state, ch);
        }
        self.accepting[state]
    }
}

fn merge_transition_maps(
    left: &RangeMapBlaze<char, StateId>,
    right: &RangeMapBlaze<char, StateId>,
) -> RangeMapBlaze<char, (StateId, StateId)> {
    let mut merged = Vec::new();
    for (left_range, left_value) in left.range_values() {
        let left_set = RangeSetBlaze::from_iter([left_range.clone()]);
        let overlap_right = right & &left_set;
        for (overlap_range, right_value) in overlap_right.range_values() {
            merged.push((overlap_range, (*left_value, *right_value)));
        }
    }

    RangeMapBlaze::from_iter(merged)
}

fn main() {
    let lower_case = TinyRegex::new(RangeSetBlaze::from_iter(['a'..='z']));
    let upper_case = TinyRegex::new(RangeSetBlaze::from_iter(['A'..='Z']));
    let letter = lower_case.union(&upper_case);

    assert!(lower_case.is_match("a"));
    assert!(!lower_case.is_match("A"));

    assert!(upper_case.is_match("A"));
    assert!(!upper_case.is_match("a"));

    assert!(letter.is_match("a"));
    assert!(letter.is_match("Z"));
    assert!(!letter.is_match(""));
    assert!(!letter.is_match("ab"));
    assert!(!letter.is_match("7"));
    assert!(!letter.is_match("é"));

    println!("All tests passed!");
}
