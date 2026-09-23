import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import { join, relative } from "node:path";
import { Window } from "happy-dom";
import { createMarkdownRenderer } from "vitepress";
import pkg from "../package.json" with { type: "json" };
import config from "../docs/.vitepress/config.mts";
import {
  documentationVersion,
  documentationVersionPlugin,
} from "../docs/.vitepress/version.ts";

const docsDirectory = new URL("../docs", import.meta.url).pathname;

async function collectMarkdownFiles(directory) {
  const files = [];

  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await collectMarkdownFiles(path)));
    } else if (entry.name.endsWith(".md")) {
      files.push(path);
    }
  }

  return files;
}

function installBrowserGlobals() {
  const window = new Window();
  globalThis.window = window;
  globalThis.document = window.document;
  globalThis.navigator = window.navigator;
  globalThis.Element = window.Element;
  globalThis.HTMLElement = window.HTMLElement;
  globalThis.SVGElement = window.SVGElement;
}

async function testMermaidDiagrams() {
  installBrowserGlobals();
  const { default: mermaid } = await import("mermaid");
  const files = await collectMarkdownFiles(docsDirectory);
  let diagramCount = 0;

  for (const file of files) {
    const markdown = await readFile(file, "utf8");
    const diagrams = markdown.matchAll(/```mermaid\s*\n([\s\S]*?)```/g);

    for (const [index, match] of Array.from(diagrams).entries()) {
      await assert.doesNotReject(
        mermaid.parse(match[1]),
        `${relative(docsDirectory, file)} diagram ${index + 1} must contain valid Mermaid syntax`,
      );
      diagramCount += 1;
    }
  }

  assert(diagramCount > 0, "The documentation must contain a Mermaid diagram.");
  console.log(`Validated ${diagramCount} Mermaid diagrams.`);
}

async function testMermaidViewer() {
  const markdown = await createMarkdownRenderer("docs", config.markdown);
  const rendered = await markdown.render(`\`\`\`mermaid
graph TD
  A --> B
\`\`\``);

  assert(
    rendered.includes("<Mermaid"),
    `Mermaid fences must use the interactive viewer component. Received:\n${rendered}`,
  );
  assert(
    !rendered.includes('data-processed="false"'),
    "Mermaid fences must not use the legacy static renderer.",
  );
  console.log("Mermaid fences use the interactive zoom viewer.");
}

async function testDocumentationVersion() {
  assert.equal(documentationVersion, pkg.version);

  const source = `---
description: "Dopbase {{version}} documentation."
---

# Available in {{version}}

Dopbase v{{version}} is installed.

\`\`\`text
Version {{version}}
\`\`\`
`;

  const plugin = documentationVersionPlugin();
  assert.equal(typeof plugin.transform, "function");

  const result = plugin.transform(source, "/docs/version-check.md?test");
  assert(result && typeof result === "object" && "code" in result);
  assert(!result.code.includes("{{version}}"));
  assert(result.code.includes(`Dopbase v${pkg.version} is installed.`));
  assert.equal(plugin.transform(source, "/docs/version-check.vue"), null);

  const env = {};
  const markdown = await createMarkdownRenderer("docs", config.markdown);
  const rendered = await markdown.render(result.code, env);

  assert.equal(
    env.frontmatter.description,
    `Dopbase ${pkg.version} documentation.`,
  );
  assert(rendered.includes(`Available in ${pkg.version}`));
  assert(rendered.includes(`Dopbase v${pkg.version} is installed.`));
  assert(rendered.includes(`Version ${pkg.version}`));
  console.log(`Documentation placeholders resolve to ${pkg.version}.`);
}

await testMermaidDiagrams();
await testMermaidViewer();
await testDocumentationVersion();
