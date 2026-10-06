use range_map_regex::declare::fsm;

fsm! {
    pub Ident {
        start: Start,
        Start { on: [('a'..='z', Word), ('x'..='x', Start)] },
        Word { accept: true },
    }
}

fn main() {}
