use fsm_demo::fsm;

fsm! {
    pub Ident {
        start: Start,
        Start { on: [('a'..='z', Wrod)] },
        Word { accept: true },
    }
}

fn main() {}
