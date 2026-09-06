import { readFile } from "node:fs/promises";
import { join } from "node:path";
import mermaid from "mermaid";
import { describe, expect, it } from "vitest";

async function readMermaidDiagram(path: string): Promise<string> {
  const markdown = await readFile(join(process.cwd(), path), "utf8");
  const match = markdown.match(/```mermaid\n([\s\S]*?)```/);
  if (match?.[1] == null)
    throw new Error(`No Mermaid diagram found in ${path}`);
  return match[1];
}

describe.each([
  ["introduction", "docs/guide/index.md"],
  ["server and client", "docs/guide/server-client.md"],
  ["runtime secret fetch", "docs/guide/run-an-application.md"],
])("%s diagram", (_name, path) => {
  it("contains valid Mermaid syntax", async () => {
    const diagram = await readMermaidDiagram(path);
    await expect(mermaid.parse(diagram)).resolves.toBeTruthy();
  });
});
