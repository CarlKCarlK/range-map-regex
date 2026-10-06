use fsm_demo::fsm;

fsm! {
    pub Ident {
        start: Begin,
        Start { on: [('a'..='z', Word)] },
        Word { accept: true },
    }
}

fn main() {}
