// Post-build step for static hosts (e.g. Bunny Storage) that don't rewrite
// extensionless URLs to .html files. Restructures the VitePress output from
// `cli/client-connect.html` to `cli/client-connect/index.html`, so that
// cleanUrls-style links (`/cli/client-connect`) resolve natively on the host.
import { mkdirSync, readdirSync, renameSync } from "node:fs";
import { join, dirname, relative } from "node:path";

const dist = new URL("../docs/.vitepress/dist", import.meta.url).pathname;

function collectHtmlFiles(dir, out = []) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      collectHtmlFiles(path, out);
    } else if (entry.name.endsWith(".html")) {
      out.push(path);
    }
  }
  return out;
}

let moved = 0;
for (const file of collectHtmlFiles(dist)) {
  const rel = relative(dist, file);
  const base = rel.split("/").pop();

  // Directory indexes and the error page are already served correctly.
  if (base === "index.html" || base === "404.html") continue;

  const targetDir = join(dist, rel.slice(0, -".html".length));
  mkdirSync(targetDir, { recursive: true });
  renameSync(file, join(targetDir, "index.html"));
  console.log(`restructured: ${rel} -> ${rel.replace(/\.html$/, "")}/index.html`);
  moved++;
}
console.log(`docs post-build: moved ${moved} pages to directory-style paths`);
