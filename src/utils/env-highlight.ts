/**
 * Line tokenizer for the `.env` editor's highlight overlay.
 *
 * Pure presentation: tokens must concatenate back to the exact line text so
 * the overlay stays pixel-aligned with the transparent textarea above it.
 * Authoritative validation comes from `parseEnvFileLines`; malformed lines
 * here merely collapse into a single "error" token.
 */

export type EnvTokenType =
  | "plain"
  | "comment"
  | "export"
  | "key"
  | "equals"
  | "value"
  | "quote"
  | "error";

export interface EnvToken {
  text: string;
  type: EnvTokenType;
}

const KEY_START = /^[A-Za-z_]/;
const KEY_CHARS = /^[A-Za-z0-9_.]*/;

function whitespace(line: string, start: number): EnvToken | null {
  const match = /^\s+/.exec(line.slice(start));
  return match ? { text: match[0], type: "plain" } : null;
}

/** Tokenizes a value part: quoted spans, plain text, and trailing comments. */
function tokenizeValue(rawValue: string): EnvToken[] {
  const tokens: EnvToken[] = [];
  const leading = /^\s*/.exec(rawValue)?.[0] ?? "";
  if (leading) tokens.push({ text: leading, type: "plain" });
  const rest = rawValue.slice(leading.length);
  if (rest === "") return tokens;

  const quote = rest[0];
  if (quote === '"' || quote === "'") {
    const closing = rest.lastIndexOf(quote);
    if (
      closing > 0 &&
      (closing === rest.length - 1 ||
        rest.slice(closing).trimStart().startsWith("#"))
    ) {
      tokens.push({ text: rest.slice(0, closing + 1), type: "quote" });
      const trailing = rest.slice(closing + 1);
      if (trailing.trimStart().startsWith("#")) {
        tokens.push({ text: trailing, type: "comment" });
      }
      return tokens;
    }
    // Unterminated quote — the whole value is flagged for the error line.
    tokens.push({ text: rest, type: "error" });
    return tokens;
  }

  const hashIndex = rest.indexOf(" #");
  if (hashIndex === -1) {
    tokens.push({ text: rest, type: "value" });
    return tokens;
  }
  if (hashIndex > 0)
    tokens.push({ text: rest.slice(0, hashIndex), type: "value" });
  tokens.push({ text: rest.slice(hashIndex), type: "comment" });
  return tokens;
}

/**
 * Tokenizes a single `.env` line. Blank lines yield an empty token list;
 * comments, assignments, and malformed lines follow the same grammar as
 * `parseEnvFileLines`.
 */
export function highlightEnvLine(line: string): EnvToken[] {
  const trimmed = line.trim();
  if (trimmed === "") return line === "" ? [] : [{ text: line, type: "plain" }];
  if (trimmed.startsWith("#")) return [{ text: line, type: "comment" }];

  const tokens: EnvToken[] = [];
  let index = 0;
  const leading = whitespace(line, 0);
  if (leading) {
    tokens.push(leading);
    index = leading.text.length;
  }
  if (line.startsWith("export ", index)) {
    tokens.push({ text: "export ", type: "export" });
    index += 7;
  }
  if (!KEY_START.test(line[index] ?? "")) {
    tokens.push({ text: line.slice(index), type: "error" });
    return tokens;
  }
  const key = KEY_CHARS.exec(line.slice(index))?.[0] ?? "";
  tokens.push({ text: key, type: "key" });
  index += key.length;
  const gap = whitespace(line, index);
  if (gap) {
    tokens.push(gap);
    index += gap.text.length;
  }
  if (line[index] !== "=") {
    return [{ text: line, type: "error" }];
  }
  tokens.push({ text: "=", type: "equals" });
  tokens.push(...tokenizeValue(line.slice(index + 1)));
  return tokens;
}

/** Tokenizes full `.env` content into one token list per line. */
export function highlightEnvContent(content: string): EnvToken[][] {
  return content.split("\n").map(highlightEnvLine);
}
