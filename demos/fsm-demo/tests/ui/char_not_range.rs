use fsm_demo::fsm;

fsm! {
    pub Ident {
        start: Start,
        Start { on: [('a'..='z', Word), ('_', Word)] },
        Word { accept: true },
    }
}

fn main() {}
