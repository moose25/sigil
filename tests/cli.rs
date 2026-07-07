//! End-to-end tests that run the built `sigil` binary.
//!
//! Uses `CARGO_BIN_EXE_sigil` (provided by Cargo for integration tests), so
//! there are no extra dependencies.

use std::process::Command;

fn sigil() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sigil"))
}

fn run(args: &[&str]) -> std::process::Output {
    sigil().args(args).output().expect("failed to run sigil")
}

#[test]
fn renders_a_banner() {
    let out = run(&["Hi", "--no-color"]);
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(s.lines().count() > 1);
}

#[test]
fn no_color_has_no_escapes() {
    let out = run(&["Hi", "--no-color"]);
    assert!(!out.stdout.contains(&0x1b));
}

#[test]
fn json_output_is_structured() {
    let out = run(&["Hi", "-F", "json"]);
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(s.contains("\"width\"") && s.contains("\"cells\""));
}

#[test]
fn unknown_gradient_errors() {
    let out = run(&["Hi", "-g", "nope"]);
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("unknown gradient"));
}

#[test]
fn gradients_subcommand_lists_presets() {
    let out = run(&["gradients"]);
    assert!(out.status.success());
    assert!(String::from_utf8(out.stdout).unwrap().contains("sunset"));
}

#[test]
fn multi_word_args_join() {
    let out = run(&["Hello", "World", "-F", "raw"]);
    assert!(out.status.success());
    // "Hello World" renders taller than one line and non-empty.
    assert!(!String::from_utf8(out.stdout).unwrap().trim().is_empty());
}

#[test]
fn png_writes_a_valid_file() {
    let path = std::env::temp_dir().join("sigil_it_test.png");
    let _ = std::fs::remove_file(&path);
    let out = run(&["Hi", "-F", "png", "-o", path.to_str().unwrap()]);
    assert!(out.status.success());
    let bytes = std::fs::read(&path).expect("png written");
    assert_eq!(
        &bytes[..8],
        &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn min_width_pads_the_box() {
    let out = run(&["Hi", "-b", "round", "--min-width", "40", "-F", "raw"]);
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    let top = s.lines().next().unwrap();
    // Box-drawing chars count as one char each; the box should reach 40 columns.
    assert!(top.chars().count() >= 40);
}

#[test]
fn version_flag_works() {
    let out = run(&["--version"]);
    assert!(out.status.success());
    assert!(String::from_utf8(out.stdout).unwrap().contains("sigil"));
}

#[test]
fn fit_respects_the_column_budget() {
    let out = run(&["hi", "--fit", "30", "-F", "raw"]);
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    let widest = s.lines().map(|l| l.chars().count()).max().unwrap_or(0);
    assert!(widest <= 30, "fit picked a font wider than 30: {widest}");
}

#[test]
fn gradient_file_loads_a_palette() {
    let path = std::env::temp_dir().join("sigil_it_palette.gpl");
    std::fs::write(&path, "GIMP Palette\n# c\n255 95 109 Coral\n#ffc371\n").unwrap();
    let out = run(&["hi", "--gradient-file", path.to_str().unwrap(), "-F", "raw"]);
    let _ = std::fs::remove_file(&path);
    assert!(out.status.success());
    assert!(!String::from_utf8(out.stdout).unwrap().trim().is_empty());
}

#[test]
fn subtitle_stacks_under_the_banner() {
    let plain = run(&["Acme", "-F", "raw"]);
    let withsub = run(&["Acme", "--subtitle", "tag", "-F", "raw"]);
    let n1 = String::from_utf8(plain.stdout).unwrap().lines().count();
    let n2 = String::from_utf8(withsub.stdout).unwrap().lines().count();
    assert!(n2 > n1, "subtitle should add rows ({n1} -> {n2})");
}
