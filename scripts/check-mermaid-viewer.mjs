import { createMarkdownRenderer } from "vitepress";

const configUrl = new URL("../docs/.vitepress/config.mts", import.meta.url);
const { default: config } = await import(configUrl.href);

const markdown = await createMarkdownRenderer("docs", config.markdown);
const rendered = await markdown.render(`\`\`\`mermaid
graph TD
  A --> B
\`\`\``);

if (!rendered.includes("<Mermaid")) {
  throw new Error(
    `Mermaid fences must use the interactive viewer component. Received:\n${rendered}`,
  );
}

if (rendered.includes('data-processed="false"')) {
  throw new Error("Mermaid fences must not use the legacy static renderer.");
}

console.log("Mermaid fences use the interactive zoom viewer.");
