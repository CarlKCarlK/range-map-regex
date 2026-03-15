use std::io;

use range_map_regex::dfa::Dfa;
use range_map_regex::display::display_char;

fn main() {
    if let Err(err) = inner_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn inner_main() -> io::Result<()> {
    let xid_start = Dfa::xid_start();
    let xid_continue = Dfa::xid_continue();
    let underscore = Dfa::from_char('_');

    // Rust identifier token rule (Unicode-aware):
    //   XID_Start XID_Continue* | _ XID_Continue+
    let ident_from_start = xid_start.concat(&xid_continue.star());
    let ident_from_underscore = underscore.concat(&xid_continue.concat(&xid_continue.star()));
    let ident = ident_from_start.union(&ident_from_underscore);
    let not_ident = ident.complement();

    assert!(ident.is_match("hello"));
    assert!(ident.is_match("x"));
    assert!(ident.is_match("_tmp"));
    assert!(ident.is_match("Var123"));
    assert!(ident.is_match("snake_case_2"));
    assert!(ident.is_match("éclair"));
    assert!(ident.is_match("变量"));
    assert!(ident.is_match("Δx"));
    assert!(ident.is_match("__"));
    assert!(!ident.is_match(""));
    assert!(!ident.is_match("_"));
    assert!(!ident.is_match("1abc"));
    assert!(!ident.is_match("a-b"));
    assert!(!ident.is_match("🤖"));

    assert!(!not_ident.is_match("hello"));
    assert!(!not_ident.is_match("éclair"));
    assert!(!not_ident.is_match("变量"));
    assert!(!not_ident.is_match("Δx"));
    assert!(not_ident.is_match(""));
    assert!(not_ident.is_match("_"));
    assert!(not_ident.is_match("1abc"));
    assert!(not_ident.is_match("a-b"));
    assert!(not_ident.is_match("🤖"));

    // display_char(&ident)?;
    display_char(&not_ident)?;

    let not_ident = not_ident.minimize();
    display_char(&not_ident)?;

    println!("Identifier showcase passed!");
    Ok(())
}
