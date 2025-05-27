use clap::CommandFactory;
use clix::cli::app::CliArgs;
use std::fs;

fn normalize(s: String) -> String {
    s.lines().map(|l| l.trim()).collect::<Vec<_>>().join("\n")
}

#[test]
fn cli_help_snapshot() {
    let mut cmd = CliArgs::command();
    cmd = cmd.term_width(120);
    let mut buf = Vec::new();
    cmd.write_long_help(&mut buf).unwrap();
    let output_raw = String::from_utf8(buf).unwrap();
    let truncated = output_raw.split("Options:").next().unwrap_or(&output_raw);
    let help_output = normalize(truncated.to_string());

    let expected = normalize(
        fs::read_to_string("tests/snapshots/cli_help.txt").expect("Missing snapshot file"),
    );

    pretty_assertions::assert_eq!(expected, help_output);
}
