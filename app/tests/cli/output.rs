use app::cli::commands::remove_one_line_ending;

#[test]
fn interactive_stdin_removes_one_line_ending_only() {
  for (input, expected) in [
    ("secret\n", "secret"),
    ("secret\r\n", "secret"),
    ("line one\nline two\n", "line one\nline two"),
    ("secret  \n", "secret  "),
    ("", ""),
  ] {
    let mut value = input.to_owned();
    remove_one_line_ending(&mut value);
    assert_eq!(value, expected);
  }
}
