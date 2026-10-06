use range_map_regex::declare::dfa;

dfa! {
    pub Number {
        start: Digits,
        Digit { is: chars('0'..='9') },
        Digits { is: many1(digti()) },
    }
}

fn main() {}
