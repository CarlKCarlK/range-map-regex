use range_map_regex::declare::dfa;

dfa! {
    pub Number {
        start: Number,
        Digits { is: many1(chars('0'..='9')) },
    }
}

fn main() {}
