use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::dfa::Dfa;
use range_set_blaze::{RangeMapBlaze, RangeSetBlaze};

pub fn display_dfa(dfa: &Dfa) -> io::Result<()> {
    display_transitions(
        dfa.start_state(),
        dfa.transitions(),
        |index| dfa.is_accepting_index(index),
        |state| state.id(),
    )
}

fn display_transitions<State, FAcceptByIndex, FIndex>(
    start: State,
    transitions: &[RangeMapBlaze<char, State>],
    is_accepting_by_index: FAcceptByIndex,
    state_index: FIndex,
) -> io::Result<()>
where
    State: Copy + Eq,
    FAcceptByIndex: Fn(usize) -> bool,
    FIndex: Fn(State) -> usize,
{
    let mut dot = String::from("digraph TinyRegex {\n  rankdir=LR;\n");
    dot.push_str("  __start [shape=point];\n");
    dot.push_str(&format!("  __start -> s{};\n", state_index(start)));
    let mut legend_entries: Vec<(String, String)> = Vec::new();

    for state_index_value in 0..transitions.len() {
        let is_accept = is_accepting_by_index(state_index_value);
        let shape = if is_accept { "doublecircle" } else { "circle" };
        dot.push_str(&format!("  s{state_index_value} [shape={shape}];\n"));
    }

    for (from_index, state_transitions) in transitions.iter().enumerate() {
        let mut ranges_by_destination: Vec<Vec<std::ops::RangeInclusive<char>>> =
            vec![Vec::new(); transitions.len()];
        for (range, to_state) in state_transitions.range_values() {
            let to_index = state_index(*to_state);
            ranges_by_destination[to_index].push(range);
        }

        for (to_index, ranges) in ranges_by_destination.into_iter().enumerate() {
            if !ranges.is_empty() {
                let first_char = ranges
                    .iter()
                    .map(|range| *range.start())
                    .min()
                    .expect("non-empty ranges has minimum start char");
                let full_set = RangeSetBlaze::from_iter(ranges);
                let full_set_text = format_char_extremes(&full_set.to_string());
                let label = if let Some((existing_short_label, _)) = legend_entries
                    .iter()
                    .find(|(_, existing_full_set)| *existing_full_set == full_set_text)
                {
                    existing_short_label.clone()
                } else {
                    let superscript_index = legend_entries.len() + 1;
                    let short_label = short_edge_label(first_char, superscript_index);
                    legend_entries.push((short_label.clone(), full_set_text));
                    short_label
                };
                dot.push_str(&format!("  s{from_index} -> s{to_index} [label=\"{label}\"];\n"));
            }
        }
    }

    if !legend_entries.is_empty() {
        let legend_label = build_legend_html(&legend_entries);
        dot.push_str("  labelloc=\"b\";\n");
        dot.push_str("  labeljust=\"l\";\n");
        dot.push_str(&format!("  label=<{legend_label}>;\n"));
    }

    dot.push_str("}\n");

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_millis();
    let base_name = format!("tiny_regex_{stamp}");
    let dot_path: PathBuf = std::env::temp_dir().join(format!("{base_name}.dot"));
    let svg_path: PathBuf = std::env::temp_dir().join(format!("{base_name}.svg"));

    fs::write(&dot_path, dot)?;

    let dot_status = Command::new("dot")
        .arg("-Tsvg")
        .arg(&dot_path)
        .arg("-o")
        .arg(&svg_path)
        .status()
        .map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "failed to run `dot` (install Graphviz). DOT file: {}",
                    dot_path.display()
                ),
            )
        })?;
    if !dot_status.success() {
        return Err(io::Error::other("`dot` failed to render graph"));
    }

    // TODO0 This opener order may be overly WSL-specific; make it configurable.
    let open_status = Command::new("wslview").arg(&svg_path).status();
    let open_status = match open_status {
        Ok(status) if status.success() => status,
        _ => Command::new("xdg-open")
            .arg(&svg_path)
            .status()
            .map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "failed to run `wslview` or `xdg-open`. Install `wslu` (for WSL) \
or `xdg-utils`. SVG file: {}",
                        svg_path.display()
                    ),
                )
            })?,
    };
    if !open_status.success() {
        return Err(io::Error::other("viewer command failed to open graph"));
    }

    Ok(())
}

fn escape_dot_char_label(ch: char) -> String {
    match ch {
        '"' => "\\\"".to_owned(),
        '\\' => "\\\\".to_owned(),
        '\0' => "\\\\0".to_owned(),
        '\n' => "\\\\n".to_owned(),
        '\r' => "\\\\r".to_owned(),
        '\t' => "\\\\t".to_owned(),
        _ if ch.is_control() => format!("U+{:04X}", ch as u32),
        _ => ch.to_string(),
    }
}

fn superscript_number(number: usize) -> String {
    const SUPERSCRIPTS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    number
        .to_string()
        .chars()
        .map(|ch| {
            let digit = ch
                .to_digit(10)
                .expect("superscript_number called with decimal digits only");
            SUPERSCRIPTS[digit as usize]
        })
        .collect()
}

fn short_edge_label(first_char: char, superscript_index: usize) -> String {
    format!(
        "{}{}",
        escape_dot_char_label(first_char),
        superscript_number(superscript_index)
    )
}

fn build_legend_html(legend_entries: &[(String, String)]) -> String {
    let mut html = String::from(
        "<TABLE BORDER=\"0\" CELLBORDER=\"0\" CELLPADDING=\"2\" CELLSPACING=\"0\">",
    );
    html.push_str("<TR><TD ALIGN=\"LEFT\"><B>Key</B></TD></TR>");
    for (short_label, range_set) in legend_entries {
        let entry = format!("{short_label} = {range_set}");
        html.push_str(&format!(
            "<TR><TD ALIGN=\"LEFT\"><FONT FACE=\"monospace\">{}</FONT></TD></TR>",
            escape_html_label(&entry)
        ));
    }
    html.push_str("</TABLE>");
    html
}

fn escape_html_label(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn format_char_extremes(text: &str) -> String {
    text.replace("'\\u{10ffff}'", "char::MAX")
}
