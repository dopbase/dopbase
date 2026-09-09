use app::cli::commands::{remove_one_line_ending, render_fields, render_table};

#[test]
fn table_output_is_borderless_and_aligned() {
  let rendered = render_table(
    &["NAME", "ID", "UPDATED"],
    &[
      vec![
        "billing".into(),
        "proj_01".into(),
        "2026-09-09 10:42 UTC".into(),
      ],
      vec!["web".into(), "proj_02".into(), "never".into()],
    ],
  );

  assert_eq!(
    rendered,
    "NAME      ID        UPDATED\nbilling   proj_01   2026-09-09 10:42 UTC\nweb       proj_02   never"
  );
}

#[test]
fn field_output_aligns_values() {
  assert_eq!(
    render_fields(&[
      ("Name:", "billing".into()),
      ("Project ID:", "proj_01".into())
    ]),
    "Name:        billing\nProject ID:  proj_01"
  );
}

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
