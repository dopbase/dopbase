import type { Plugin } from "vite";
import pkg from "../../package.json" with { type: "json" };

const versionPlaceholder = "{{version}}";

export const documentationVersion = pkg.version;
export const documentationVersionLabel = `v${documentationVersion}`;

export function replaceVersionPlaceholder(source: string): string {
  return source.replaceAll(versionPlaceholder, documentationVersion);
}

export function documentationVersionPlugin(): Plugin {
  return {
    name: "dopbase-docs-version",
    enforce: "pre",
    transform(source, id) {
      const path = id.replace(/\?.*$/, "");
      if (!path.endsWith(".md")) return null;

      const code = replaceVersionPlaceholder(source);
      return code === source ? null : { code, map: null };
    },
  };
}
