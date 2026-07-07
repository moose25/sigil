//! Golden / regression tests: pin stable output so a rendering refactor can't
//! silently change what users see. Uses the built binary (no extra deps).

use std::process::Command;

fn sigil() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sigil"))
}

fn stdout(args: &[&str]) -> String {
    let out = sigil().args(args).output().expect("run sigil");
    assert!(out.status.success(), "sigil {args:?} failed");
    String::from_utf8(out.stdout).expect("utf8")
}

#[test]
fn raw_hi_is_byte_exact() {
    // The standard font is embedded and deterministic, so this exact rendering
    // is a stable golden. If it changes, it should change on purpose.
    let expected = "  _   _   _ \n | | | | (_)\n | |_| | | |\n |  _  | | |\n |_| |_| |_|\n";
    assert_eq!(stdout(&["Hi", "-F", "raw"]), expected);
}

#[test]
fn json_shape_is_stable() {
    let s = stdout(&["A", "-F", "json"]);
    assert!(s.contains("\"width\""));
    assert!(s.contains("\"height\""));
    assert!(s.contains("\"cells\""));
    assert!(s.contains("\"char\""));
    assert!(s.contains("\"color\""));
}

#[test]
fn svg_is_self_contained() {
    let s = stdout(&["Hi", "-g", "sunset", "-F", "svg"]);
    assert!(s.starts_with("<svg"));
    assert!(s.trim_end().ends_with("</svg>"));
    assert!(s.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(s.contains("fill=\"#"));
}

#[test]
fn markdown_is_a_fenced_block() {
    let s = stdout(&["Hi", "-F", "markdown"]);
    assert!(s.starts_with("```\n"));
    assert!(s.trim_end().ends_with("```"));
    // No ANSI escapes inside a markdown fence.
    assert!(!s.contains('\u{1b}'));
}

#[test]
fn mark_is_deterministic() {
    let a = stdout(&["mark", "acme", "-g", "aurora"]);
    let b = stdout(&["mark", "acme", "-g", "aurora"]);
    let c = stdout(&["mark", "acme2", "-g", "aurora"]);
    assert_eq!(a, b, "same seed must yield the same mark");
    assert_ne!(a, c, "different seed should differ");
}

#[test]
fn shade_reads_without_color() {
    // With --no-color, shade mode still conveys the gradient via block glyphs.
    let s = stdout(&["Hi", "--fill", "shade", "--no-color", "-F", "raw"]);
    assert!(s.chars().any(|c| "░▒▓█".contains(c)));
    assert!(!s.contains('\u{1b}'));
}

#[test]
fn composed_features_still_render() {
    // Several independent features at once must not fight each other.
    let s = stdout(&[
        "Acme",
        "--from",
        "#ff5f6d",
        "--subtitle",
        "ship it",
        "-d",
        "radial",
        "--icon",
        "*",
        "-F",
        "raw",
    ]);
    assert!(s.lines().count() > 4);
    assert!(s.contains('*')); // the icon survived the pipeline
}
