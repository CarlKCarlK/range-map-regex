use range_map_regex::dfa::Dfa;

#[test]
fn basic_example_behavior() {
    let hello = Dfa::string("hello");
    assert!(hello.is_match("hello"));
    assert!(!hello.is_match("hell"));
    assert!(!hello.is_match("hello!"));

    let maybe_hi = Dfa::string("hi").optional();
    assert!(maybe_hi.is_match(""));
    assert!(maybe_hi.is_match("hi"));
    assert!(!maybe_hi.is_match("h"));

    let one_or_more_a = Dfa::from_char_range('a'..='a').plus();
    assert!(one_or_more_a.is_match("a"));
    assert!(one_or_more_a.is_match("aaaa"));
    assert!(!one_or_more_a.is_match(""));
    assert!(!one_or_more_a.is_match("b"));

    let empty = Dfa::empty();
    assert!(!empty.is_match(""));
    assert!(!empty.is_match("a"));

    let epsilon = Dfa::epsilon();
    assert!(epsilon.is_match(""));
    assert!(!epsilon.is_match("a"));

    let lower_case = Dfa::from_char_range('a'..='z');
    let upper_case = Dfa::from_char_range('A'..='Z');

    assert!(lower_case.is_match("a"));
    assert!(!lower_case.is_match("A"));
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
}

#[test]
fn not_ident_example_behavior() {
    let xid_start = Dfa::xid_start();
    let xid_continue = Dfa::xid_continue();
    let underscore = Dfa::from_char_range('_'..='_');

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

    let minimized = not_ident.minimize();
    assert!(minimized.is_match(""));
    assert!(minimized.is_match("_"));
    assert!(minimized.is_match("1abc"));
    assert!(!minimized.is_match("hello"));
    assert!(!minimized.is_match("éclair"));
}
