pub const ADMIN_UI_NOT_EMBEDDED_PAGE: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Dopbase development server</title>
  </head>
  <body>
    <main>
      <h1>Dopbase backend-only development build</h1>
      <p>The REST API is running, but the Admin UI is not embedded.</p>
      <p>Run <code>bun run dev</code> for Vue development.</p>
      <p>Run <code>bun run build:binary</code> to build the single executable.</p>
    </main>
  </body>
</html>
"#;
