use range_map_regex::declare::dfa;

dfa! {
    pub Number {
        start: Digits,
        Digits { is: "[0-9]+" },
    }
}

fn main() {}
