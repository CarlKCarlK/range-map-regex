use fsm_demo::fsm;

fsm! {
    pub Ident {
        start: Start,
        Start { on: [('a'..='z', Word), ('x'..='x', Start)] },
        Word { accept: true },
    }
}

fn main() {}
