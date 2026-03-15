use std::io;

use range_map_regex::dfa::Dfa;
use range_map_regex::display::display_byte;
use range_map_regex::display::display_char;

fn main() {
    if let Err(err) = inner_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn inner_main() -> io::Result<()> {
    // Start tiny: exactly one ASCII character.
    let only_a = Dfa::string("a");
    let only_a_step1 = only_a.minimize();
    display_char(&only_a_step1)?;

    let only_a_step2 = only_a_step1.to_utf8_dfa();
    let only_a_step2_min = only_a_step2.minimize();
    assert!(only_a.is_match("a"));
    assert!(only_a_step2.is_match_bytes(b"a"));
    assert!(!only_a_step2.is_match_bytes(b""));
    assert!(!only_a_step2.is_match_bytes(b"aa"));
    assert!(!only_a_step2.is_match_bytes(b"b"));

    println!(
        "\"a\": step1(char minimize)={}, step2(utf8 byte dfa)={}",
        only_a_step1.state_count(),
        only_a_step2.state_count()
    );
    println!(
        "\"a\": step2 minimized(byte dfa)={}",
        only_a_step2_min.state_count()
    );
    display_byte(&only_a_step2_min)?;

    // Slightly richer: one ASCII and one multi-byte UTF-8 character.
    let a_or_e_acute = Dfa::string("a").union(&Dfa::from_char('é'));
    let a_or_e_acute_step1 = a_or_e_acute.minimize();
    let a_or_e_acute_step2 = a_or_e_acute_step1.to_utf8_dfa();
    let a_or_e_acute_step2_min = a_or_e_acute_step2.minimize();
    assert!(a_or_e_acute.is_match("a"));
    assert!(a_or_e_acute.is_match("é"));
    assert!(a_or_e_acute_step2.is_match_bytes("a".as_bytes()));
    assert!(a_or_e_acute_step2.is_match_bytes("é".as_bytes()));
    assert!(!a_or_e_acute_step2.is_match_bytes("e".as_bytes()));
    assert!(!a_or_e_acute_step2.is_match_bytes(&[0xC3])); // invalid UTF-8 prefix only

    println!(
        "\"a\"|\"é\": step1(char minimize)={}, step2(utf8 byte dfa)={}",
        a_or_e_acute_step1.state_count(),
        a_or_e_acute_step2.state_count()
    );
    println!(
        "\"a\"|\"é\": step2 minimized(byte dfa)={}",
        a_or_e_acute_step2_min.state_count()
    );
    display_byte(&a_or_e_acute_step2_min)?;

    println!("char_to_byte example passed!");
    Ok(())
}
