import { extname } from "node:path";

interface AssetFileNameInfo {
  names: string[];
  originalFileNames: string[];
}

const fontExtensions = new Set([
  ".eot",
  ".otf",
  ".ttc",
  ".ttf",
  ".woff",
  ".woff2",
]);

export function assetFileNames(asset: AssetFileNameInfo): string {
  const sourceName = asset.names[0] ?? asset.originalFileNames[0] ?? "";
  const extension = extname(sourceName).toLowerCase();

  return fontExtensions.has(extension)
    ? "assets/font/[name][extname]"
    : "assets/[name]-[hash][extname]";
}
