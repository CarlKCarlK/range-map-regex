use std::io;

use range_map_regex::dfa::Dfa;

fn main() {
    if let Err(err) = inner_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn inner_main() -> io::Result<()> {
    let lower_case = Dfa::from_accept_set('a'..='z');

    assert!(lower_case.is_match("a"));
    assert!(!lower_case.is_match("A"));

    let upper_case = Dfa::from_accept_set('A'..='Z');
    assert!(upper_case.is_match("A"));
    assert!(!upper_case.is_match("a"));

    let letter = lower_case.union(&upper_case);
    assert!(letter.is_match("a"));
    assert!(letter.is_match("Z"));
    assert!(!letter.is_match(""));
    assert!(!letter.is_match("ab"));
    assert!(!letter.is_match("7"));
    assert!(!letter.is_match("é"));

    range_map_regex::display::display_dfa(
        letter.start_state(),
        letter.transitions(),
        |index| letter.is_accepting_index(index),
        |state| state.id(),
    )?;

    println!("All tests passed!");
    Ok(())
}
