use range_map_regex::declare::fsm;

fsm! {
    pub Ident {
        start: Start,
        Start { on: [('a'..='z', Word)] },
        Word { acept: true },
    }
}

fn main() {}
