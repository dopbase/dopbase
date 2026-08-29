import { describe, expect, it } from "vitest";
import {
  mergeLayoutValues,
  parseEnvFile,
  parseEnvFileLines,
  serializeEnvFile,
  stripLayoutValues,
  summarizeEnvEntries,
} from "./env-file";

describe("parseEnvFile", () => {
  it("parses plain KEY=value lines", () => {
    const { entries, errors } = parseEnvFile("A=1\nB=two\n");
    expect(errors).toEqual([]);
    expect(entries).toEqual([
      { key: "A", value: "1" },
      { key: "B", value: "two" },
    ]);
  });

  it("skips blank lines and comments", () => {
    const { entries, errors } = parseEnvFile(
      "\n# comment\n  # indented\nA=1\n",
    );
    expect(errors).toEqual([]);
    expect(entries).toEqual([{ key: "A", value: "1" }]);
  });

  it("supports the export prefix", () => {
    const { entries } = parseEnvFile("export DATABASE_URL=postgres://x\n");
    expect(entries).toEqual([{ key: "DATABASE_URL", value: "postgres://x" }]);
  });

  it("unquotes double-quoted values with escapes", () => {
    const { entries } = parseEnvFile('A="line1\\nline2"\nB="say \\"hi\\""\n');
    expect(entries).toEqual([
      { key: "A", value: "line1\nline2" },
      { key: "B", value: 'say "hi"' },
    ]);
  });

  it("keeps single-quoted values literally", () => {
    const { entries } = parseEnvFile("A='keep \\n literal'\n");
    expect(entries).toEqual([{ key: "A", value: "keep \\n literal" }]);
  });

  it("strips inline comments after unquoted values", () => {
    const { entries } = parseEnvFile("A=value # trailing comment\n");
    expect(entries).toEqual([{ key: "A", value: "value" }]);
  });

  it("reports malformed lines", () => {
    const { entries, errors } = parseEnvFile("=novalue\n1BAD=x\nJUSTAKEY\n");
    expect(entries).toEqual([]);
    expect(errors).toHaveLength(3);
  });

  it("reports duplicate keys and keeps the first value", () => {
    const { entries, errors } = parseEnvFile("A=1\nA=2\n");
    expect(entries).toEqual([{ key: "A", value: "1" }]);
    expect(errors).toHaveLength(1);
  });
});

describe("serializeEnvFile", () => {
  it("round-trips simple values", () => {
    const content = serializeEnvFile([
      { key: "A", value: "1" },
      { key: "B_URL", value: "postgres://x:5432/db" },
    ]);
    expect(content).toBe("A=1\nB_URL=postgres://x:5432/db\n");
  });

  it("quotes risky values", () => {
    const content = serializeEnvFile([
      { key: "A", value: 'has "quotes"' },
      { key: "B", value: "line1\nline2" },
      { key: "C", value: "" },
    ]);
    expect(content).toBe('A="has \\"quotes\\""\nB="line1\\nline2"\nC=\n');
  });
});

describe("summarizeEnvEntries", () => {
  it("previews keys and counts without values", () => {
    const summary = summarizeEnvEntries([
      { key: "A", value: "secret-value" },
      { key: "B", value: "another-secret" },
    ]);
    expect(summary).toEqual({ count: 2, keys: ["A", "B"] });
    expect(JSON.stringify(summary)).not.toContain("secret-value");
  });
});

describe("parseEnvFileLines", () => {
  it("anchors issues to their line numbers", () => {
    const { entries, issues } = parseEnvFileLines(
      "A=1\n1BAD=x\nJUSTAKEY\nA=2\n",
    );
    expect(entries).toEqual([{ key: "A", value: "1" }]);
    expect(issues).toEqual([
      { line: 2, message: 'invalid key "1BAD".' },
      { line: 3, message: "expected KEY=value." },
      { line: 4, message: 'duplicate key "A".' },
    ]);
  });

  it("reports no issues for valid content", () => {
    const { issues } = parseEnvFileLines("# app\nA=1\nexport B='two'\n");
    expect(issues).toEqual([]);
  });
});

describe("stripLayoutValues", () => {
  it("strips values while keeping comments, blanks, and ordering", () => {
    expect(stripLayoutValues("# app\nA=1\n\nB=two\n")).toBe(
      "# app\nA=\n\nB=\n",
    );
  });

  it("strips quoted values and preserves export prefixes", () => {
    expect(stripLayoutValues("export A=\"x y\"\nB='z'\n")).toBe(
      "export A=\nB=\n",
    );
  });

  it("moves inline comments onto their own line above the key", () => {
    expect(stripLayoutValues("A=1 # trailing\nB=2\n")).toBe(
      "# trailing\nA=\nB=\n",
    );
  });

  it("keeps unparsable lines untouched", () => {
    expect(stripLayoutValues("JUSTAKEY\n")).toBe("JUSTAKEY\n");
  });

  it("never contains the original values", () => {
    const layout = stripLayoutValues(
      'TOKEN=super-secret\nURL="https://x.example/a b"\n',
    );
    expect(layout).not.toContain("super-secret");
    expect(layout).not.toContain("https://x.example/a b");
  });
});

describe("mergeLayoutValues", () => {
  it("fills layout slots in place and keeps comments", () => {
    const content = mergeLayoutValues("# app\nA=\n\nB=\n", [
      { key: "A", value: "1" },
      { key: "B", value: "two" },
    ]);
    expect(content).toBe("# app\nA=1\n\nB=two\n");
  });

  it("appends keys missing from the layout", () => {
    const content = mergeLayoutValues("A=\n", [
      { key: "A", value: "1" },
      { key: "NEW", value: "x" },
    ]);
    expect(content).toBe("A=1\nNEW=x\n");
  });

  it("preserves export prefixes when filling", () => {
    expect(mergeLayoutValues("export A=\n", [{ key: "A", value: "1" }])).toBe(
      "export A=1\n",
    );
  });

  it("quotes risky values", () => {
    expect(mergeLayoutValues(null, [{ key: "A", value: 'has "quotes"' }])).toBe(
      'A="has \\"quotes\\""',
    );
  });

  it("round-trips content through strip and merge", () => {
    const original = '# app\nA=1\nB="two words"\n';
    const parsed = parseEnvFileLines(original);
    const layout = stripLayoutValues(original);
    const restored = mergeLayoutValues(layout, parsed.entries);
    expect(parseEnvFileLines(restored).entries).toEqual(parsed.entries);
    expect(layout).not.toContain("1");
    expect(layout).not.toContain("two words");
  });
});
