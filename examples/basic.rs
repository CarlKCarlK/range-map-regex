use range_set_blaze::RangeMapBlaze;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum State {
    Start = 0,
    Accept = 1,
    Dead = 2,
}

struct TinyRegex {
    transitions: [RangeMapBlaze<char, State>; 3],
}

impl TinyRegex {
    fn lowercase_ascii_letter() -> Self {
        // State machine for: exactly one char in [a-z], and nothing else.
        let start = RangeMapBlaze::from_iter([
            (char::MIN..=char::MAX, State::Dead),
            ('a'..='z', State::Accept),
        ]);
        let accept = RangeMapBlaze::from_iter([(char::MIN..=char::MAX, State::Dead)]);
        let dead = RangeMapBlaze::from_iter([(char::MIN..=char::MAX, State::Dead)]);

        Self {
            transitions: [start, accept, dead],
        }
    }

    fn step(&self, state: State, ch: char) -> State {
        *self.transitions[state as usize]
            .get(ch)
            .expect("state transitions cover all chars")
    }

    fn is_match(&self, input: &str) -> bool {
        let mut state = State::Start;
        for ch in input.chars() {
            state = self.step(state, ch);
        }
        state == State::Accept
    }
}

fn main() {
    let lower_case = TinyRegex::lowercase_ascii_letter();

    assert!(lower_case.is_match("a"));
    assert!(lower_case.is_match("z"));

    assert!(!lower_case.is_match(""));
    assert!(!lower_case.is_match("A"));
    assert!(!lower_case.is_match("ab"));
    assert!(!lower_case.is_match("7"));
    assert!(!lower_case.is_match("é"));
}
