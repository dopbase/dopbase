export interface ParsedEnvEntry {
  key: string;
  value: string;
}

export interface ParsedEnvFile {
  entries: ParsedEnvEntry[];
  /** Human-readable problems for lines that could not be parsed. */
  errors: string[];
}

/** A validation problem anchored to a specific 1-based line number. */
export interface EnvFileIssue {
  line: number;
  message: string;
}

export interface ParsedEnvLines {
  entries: ParsedEnvEntry[];
  issues: EnvFileIssue[];
}

const KEY_PATTERN = /^[A-Za-z_][A-Za-z0-9_.]*$/;

function unquote(raw: string): { value: string; quoted: boolean } | null {
  if (raw.length < 2) return null;
  const quote = raw[0];
  if (quote !== '"' && quote !== "'") return null;
  if (raw[raw.length - 1] !== quote) return null;
  const inner = raw.slice(1, -1);
  if (quote === '"') {
    return {
      value: inner
        .replace(/\\n/g, "\n")
        .replace(/\\r/g, "\r")
        .replace(/\\t/g, "\t")
        .replace(/\\"/g, '"')
        .replace(/\\\\/g, "\\"),
      quoted: true,
    };
  }
  return { value: inner, quoted: true };
}

/**
 * Parses `.env` content into entries plus per-line validation issues.
 *
 * Deliberately strict and dependency-free: handles blank lines, `#`
 * comments, an optional `export ` prefix, single- and double-quoted values,
 * and trailing `# comment` after unquoted values. Values are returned to
 * callers that need them (import submit, the editor buffer) but UI previews
 * must use {@link summarizeEnvEntries} so plaintext never renders.
 */
export function parseEnvFileLines(content: string): ParsedEnvLines {
  const entries: ParsedEnvEntry[] = [];
  const issues: EnvFileIssue[] = [];
  const seen = new Set<string>();
  const lines = content.split(/\r?\n/);

  lines.forEach((line, index) => {
    const lineNumber = index + 1;
    const trimmed = line.trim();
    if (trimmed === "" || trimmed.startsWith("#")) return;

    const withoutExport = trimmed.startsWith("export ")
      ? trimmed.slice(7).trim()
      : trimmed;
    const separator = withoutExport.indexOf("=");
    if (separator <= 0) {
      issues.push({ line: lineNumber, message: "expected KEY=value." });
      return;
    }

    const key = withoutExport.slice(0, separator).trim();
    const rawValue = withoutExport.slice(separator + 1).trim();
    if (!KEY_PATTERN.test(key)) {
      issues.push({ line: lineNumber, message: `invalid key "${key}".` });
      return;
    }
    if (seen.has(key)) {
      issues.push({ line: lineNumber, message: `duplicate key "${key}".` });
      return;
    }

    let value: string;
    const quoted = unquote(rawValue);
    if (quoted) {
      value = quoted.value;
    } else {
      // Strip an inline comment that follows an unquoted value.
      const hashIndex = rawValue.indexOf(" #");
      value = (
        hashIndex === -1 ? rawValue : rawValue.slice(0, hashIndex)
      ).trim();
      if (value.startsWith("#")) {
        issues.push({
          line: lineNumber,
          message: `key "${key}" has no value.`,
        });
        return;
      }
    }

    seen.add(key);
    entries.push({ key, value });
  });

  return { entries, issues };
}

/**
 * Parses `.env`-formatted content locally in the browser. A thin wrapper
 * over {@link parseEnvFileLines} that flattens the per-line issues into
 * human-readable strings.
 */
export function parseEnvFile(content: string): ParsedEnvFile {
  const { entries, issues } = parseEnvFileLines(content);
  return {
    entries,
    errors: issues.map((issue) => `Line ${issue.line}: ${issue.message}`),
  };
}

/** Quotes a value for `.env` output when it contains risky characters. */
function serializeValue(value: string): string {
  if (value === "") return "";
  if (/^[A-Za-z0-9_@/+=.,:%~-]*$/.test(value)) return value;
  return `"${value
    .replace(/\\/g, "\\\\")
    .replace(/"/g, '\\"')
    .replace(/\n/g, "\\n")
    .replace(/\r/g, "\\r")
    .replace(/\t/g, "\\t")}"`;
}

/** Serializes entries back into `.env` format for export downloads. */
export function serializeEnvFile(entries: ParsedEnvEntry[]): string {
  return (
    entries
      .map((entry) => `${entry.key}=${serializeValue(entry.value)}`)
      .join("\n") + (entries.length > 0 ? "\n" : "")
  );
}

/** Safe import preview: key names and counts, never values. */
export function summarizeEnvEntries(entries: ParsedEnvEntry[]): {
  count: number;
  keys: string[];
} {
  return { count: entries.length, keys: entries.map((entry) => entry.key) };
}

/**
 * Builds the persistable layout from edited `.env` content: comments, blank
 * lines, key ordering, and `export` prefixes are preserved while every
 * value is stripped, so no plaintext is ever stored. An unquoted value's
 * inline comment is moved onto its own line above the key, because a bare
 * `KEY= # comment` slot would not re-parse as a valid empty value.
 */
export function stripLayoutValues(content: string): string {
  const out: string[] = [];
  for (const line of content.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (trimmed === "" || trimmed.startsWith("#")) {
      out.push(line);
      continue;
    }
    const separator = trimmed.indexOf("=");
    if (separator <= 0) {
      // Unparsable line: kept as-is (it blocks saving anyway).
      out.push(line);
      continue;
    }
    const prefix = trimmed.slice(0, separator + 1);
    const rawValue = trimmed.slice(separator + 1).trim();
    const quoted = unquote(rawValue);
    if (quoted) {
      out.push(prefix);
      continue;
    }
    const hashIndex = rawValue.indexOf(" #");
    if (hashIndex === -1) {
      out.push(prefix);
      continue;
    }
    const comment = rawValue.slice(hashIndex).trim();
    out.push(comment.startsWith("#") ? comment : `#${comment}`);
    out.push(prefix);
  }
  return out.join("\n");
}

/**
 * Fills a stored layout with plaintext values for an editor session: every
 * `KEY=` slot in the layout receives its value re-serialized in place, keys
 * without a layout slot are appended at the end, and comments/blank lines
 * are untouched. Layout lines never carry values (see
 * {@link stripLayoutValues}), so simple slot substitution is safe.
 */
export function mergeLayoutValues(
  layout: string | null,
  entries: ParsedEnvEntry[],
): string {
  const values = new Map(entries.map((entry) => [entry.key, entry.value]));
  const out: string[] = [];
  const placed = new Set<string>();
  for (const line of layout ? layout.split(/\r?\n/) : []) {
    const trimmed = line.trim();
    let replaced = false;
    if (trimmed !== "" && !trimmed.startsWith("#")) {
      const withoutExport = trimmed.startsWith("export ")
        ? trimmed.slice(7).trim()
        : trimmed;
      const separator = withoutExport.indexOf("=");
      if (separator > 0) {
        const key = withoutExport.slice(0, separator).trim();
        const value = values.get(key);
        if (value !== undefined) {
          placed.add(key);
          const exportPrefix = trimmed.startsWith("export ") ? "export " : "";
          out.push(`${exportPrefix}${key}=${serializeValue(value)}`);
          replaced = true;
        }
      }
    }
    if (!replaced) out.push(line);
  }
  // Appended keys go before a single trailing blank line (the artifact of a
  // final newline) so they do not land after a stray empty line.
  const trailingBlank = out.length > 0 && out[out.length - 1] === "";
  if (trailingBlank) out.pop();
  for (const entry of entries) {
    if (!placed.has(entry.key)) {
      out.push(`${entry.key}=${serializeValue(entry.value)}`);
    }
  }
  if (trailingBlank) out.push("");
  return out.join("\n");
}
