use std::io;

use range_map_regex::dfa::Dfa;
use range_map_regex::display::display_dfa;

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

    let minimized_letter = letter.minimize();
    assert!(minimized_letter.state_count() < letter.state_count());
    assert!(minimized_letter.is_match("a"));
    assert!(minimized_letter.is_match("Z"));
    assert!(!minimized_letter.is_match(""));
    assert!(!minimized_letter.is_match("ab"));
    assert!(!minimized_letter.is_match("7"));
    assert!(!minimized_letter.is_match("é"));

    display_dfa(&minimized_letter)?;

    println!("All tests passed!");
    Ok(())
}
