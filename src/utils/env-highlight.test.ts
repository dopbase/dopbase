import { describe, expect, it } from "vitest";
import { highlightEnvContent, highlightEnvLine } from "./env-highlight";

/** Token shape helper: [text, type] pairs. */
function shape(line: string): Array<[string, string]> {
  return highlightEnvLine(line).map((token) => [token.text, token.type]);
}

describe("highlightEnvLine", () => {
  it("returns no tokens for a blank line", () => {
    expect(highlightEnvLine("")).toEqual([]);
  });

  it("keeps whitespace-only lines as a plain token", () => {
    expect(shape("   ")).toEqual([["   ", "plain"]]);
  });

  it("tokenizes a whole-line comment", () => {
    expect(shape("# hello world")).toEqual([["# hello world", "comment"]]);
  });

  it("tokenizes key, equals, and value", () => {
    expect(shape("DATABASE_URL=postgres://x")).toEqual([
      ["DATABASE_URL", "key"],
      ["=", "equals"],
      ["postgres://x", "value"],
    ]);
  });

  it("tokenizes the export prefix", () => {
    expect(shape("export A=1")).toEqual([
      ["export ", "export"],
      ["A", "key"],
      ["=", "equals"],
      ["1", "value"],
    ]);
  });

  it("preserves leading whitespace so the overlay stays aligned", () => {
    expect(shape("  A=1")).toEqual([
      ["  ", "plain"],
      ["A", "key"],
      ["=", "equals"],
      ["1", "value"],
    ]);
  });

  it("tokenizes quoted values", () => {
    expect(shape('A="x y"')).toEqual([
      ["A", "key"],
      ["=", "equals"],
      ['"x y"', "quote"],
    ]);
  });

  it("tokenizes trailing inline comments", () => {
    expect(shape("A=1 # note")).toEqual([
      ["A", "key"],
      ["=", "equals"],
      ["1", "value"],
      [" # note", "comment"],
    ]);
  });

  it("flags unterminated quotes as an error", () => {
    expect(shape('A="unterminated')).toEqual([
      ["A", "key"],
      ["=", "equals"],
      ['"unterminated', "error"],
    ]);
  });

  it("flags invalid key starts as a single error token", () => {
    expect(shape("1BAD=x")).toEqual([["1BAD=x", "error"]]);
  });

  it("flags missing equals signs as a single error token", () => {
    expect(shape("JUSTAKEY")).toEqual([["JUSTAKEY", "error"]]);
  });

  it("tolerates spaces around the equals sign", () => {
    expect(shape("A = 1")).toEqual([
      ["A", "key"],
      [" ", "plain"],
      ["=", "equals"],
      [" ", "plain"],
      ["1", "value"],
    ]);
  });
});

describe("highlightEnvContent", () => {
  it("tokenizes content into one list per line", () => {
    const lines = highlightEnvContent("# app\nA=1\n\nB='two'\n");
    // A trailing newline yields a final empty line, exactly like an editor.
    expect(lines).toHaveLength(5);
    expect(lines[0]).toEqual([{ text: "# app", type: "comment" }]);
    expect(lines[1].map((token) => token.type)).toEqual([
      "key",
      "equals",
      "value",
    ]);
    expect(lines[2]).toEqual([]);
    expect(lines[3].map((token) => token.type)).toEqual([
      "key",
      "equals",
      "quote",
    ]);
    expect(lines[4]).toEqual([]);
  });
});
