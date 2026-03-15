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
        LabelStyle::FirstChar,
    )
}

pub fn display_full(dfa: &Dfa) -> io::Result<()> {
    display_transitions(
        dfa.start_state(),
        dfa.transitions(),
        |index| dfa.is_accepting_index(index),
        |state| state.id(),
        LabelStyle::FullRangeSet,
    )
}

#[derive(Clone, Copy)]
enum LabelStyle {
    FirstChar,
    FullRangeSet,
}

fn display_transitions<State, FAcceptByIndex, FIndex>(
    start: State,
    transitions: &[RangeMapBlaze<char, State>],
    is_accepting_by_index: FAcceptByIndex,
    state_index: FIndex,
    label_style: LabelStyle,
) -> io::Result<()>
where
    State: Copy + Eq,
    FAcceptByIndex: Fn(usize) -> bool,
    FIndex: Fn(State) -> usize,
{
    let mut dot = String::from("digraph TinyRegex {\n  rankdir=LR;\n");
    dot.push_str("  __start [shape=point];\n");
    dot.push_str(&format!("  __start -> s{};\n", state_index(start)));

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
                let label = match label_style {
                    LabelStyle::FirstChar => {
                        let first_char = ranges
                            .iter()
                            .map(|range| *range.start())
                            .min()
                            .expect("non-empty ranges has minimum start char");
                        escape_dot_char_label(first_char)
                    }
                    LabelStyle::FullRangeSet => {
                        let full_set = RangeSetBlaze::from_iter(ranges);
                        escape_dot_text_label(&full_set.to_string())
                    }
                };
                dot.push_str(&format!("  s{from_index} -> s{to_index} [label=\"{label}\"];\n"));
            }
        }
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

fn escape_dot_text_label(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}
