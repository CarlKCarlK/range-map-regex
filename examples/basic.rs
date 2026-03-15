use std::io;

use range_map_regex::dfa::Dfa;

fn main() {
    if let Err(err) = inner_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn inner_main() -> io::Result<()> {
    let hello = Dfa::string("hello");
    assert!(hello.is_match("hello"));
    assert!(!hello.is_match("hell"));
    assert!(!hello.is_match("hello!"));

    let maybe_hi = Dfa::string("hi").optional();
    assert!(maybe_hi.is_match(""));
    assert!(maybe_hi.is_match("hi"));
    assert!(!maybe_hi.is_match("h"));

    let one_or_more_a = Dfa::from_char('a').plus();
    assert!(one_or_more_a.is_match("a"));
    assert!(one_or_more_a.is_match("aaaa"));
    assert!(!one_or_more_a.is_match(""));
    assert!(!one_or_more_a.is_match("b"));

    let empty = Dfa::empty();
    assert!(!empty.is_match(""));
    assert!(!empty.is_match("a"));
    // display_dfa(&empty)?;

    let epsilon = Dfa::epsilon();
    assert!(epsilon.is_match(""));
    assert!(!epsilon.is_match("a"));
    // display_dfa(&epsilon)?;

    let lower_case = Dfa::from_char_range('a'..='z');

    assert!(lower_case.is_match("a"));
    assert!(!lower_case.is_match("A"));

    let upper_case = Dfa::from_char_range('A'..='Z');
    assert!(upper_case.is_match("A"));
    assert!(!upper_case.is_match("a"));

    let lower_then_upper = lower_case.concat(&upper_case);
    assert!(lower_then_upper.is_match("aA"));
    assert!(lower_then_upper.is_match("zZ"));
    assert!(!lower_then_upper.is_match(""));
    assert!(!lower_then_upper.is_match("a"));
    assert!(!lower_then_upper.is_match("A"));
    assert!(!lower_then_upper.is_match("Aa"));
    assert!(!lower_then_upper.is_match("aAA"));

    let lower_star = lower_case.star();
    assert!(lower_star.is_match(""));
    assert!(lower_star.is_match("a"));
    assert!(lower_star.is_match("azby"));
    assert!(!lower_star.is_match("A"));
    assert!(!lower_star.is_match("aA"));
    assert!(!lower_star.is_match("7"));
    // display_dfa(&lower_star)?;
    // let lower_star = lower_star.minimize();
    // display_dfa(&lower_star)?;

    let empty_star = empty.star();
    assert!(empty_star.is_match(""));
    assert!(!empty_star.is_match("a"));

    let epsilon_star = epsilon.star();
    assert!(epsilon_star.is_match(""));
    assert!(!epsilon_star.is_match("a"));

    let letter = lower_case.union(&upper_case);
    let impossible = lower_case.intersection(&upper_case);
    assert!(!impossible.is_match("a"));
    assert!(!impossible.is_match("A"));
    assert!(!impossible.is_match(""));

    let not_lower_case = lower_case.complement();
    assert!(!not_lower_case.is_match("a"));
    assert!(not_lower_case.is_match("A"));
    assert!(not_lower_case.is_match(""));

    let xid_start = Dfa::xid_start();
    assert!(xid_start.is_match("a"));
    assert!(xid_start.is_match("Δ"));
    assert!(xid_start.is_match("变"));
    assert!(!xid_start.is_match("1"));
    assert!(!xid_start.is_match("_"));

    let xid_continue = Dfa::xid_continue();
    assert!(xid_continue.is_match("a"));
    assert!(xid_continue.is_match("_"));
    assert!(xid_continue.is_match("1"));
    assert!(!xid_continue.is_match("-"));

    assert!(letter.is_match("a"));
    assert!(letter.is_match("Z"));
    assert!(!letter.is_match(""));
    assert!(!letter.is_match("ab"));
    assert!(!letter.is_match("7"));
    assert!(!letter.is_match("é"));

    let minimized_letter = letter.minimize();
    assert!(minimized_letter.state_count() < letter.state_count());
    assert!(minimized_letter.is_match("a"));
    assert!(minimized_letter.is_match("Z"));
    assert!(!minimized_letter.is_match(""));
    assert!(!minimized_letter.is_match("ab"));
    assert!(!minimized_letter.is_match("7"));
    assert!(!minimized_letter.is_match("é"));

    // display_dfa(&minimized_letter)?;

    println!("All tests passed!");
    Ok(())
}
