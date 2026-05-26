import { describe, expect, it } from "vitest";

import { rank, scoreFuzzy, type CommandItem } from "./command-palette";

const noop = () => undefined;

const items: CommandItem[] = [
  { id: "1", kind: "verb", title: "Open Library", keywords: ["recordings"], action: noop },
  { id: "2", kind: "verb", title: "Start recording", keywords: ["meeting"], action: noop },
  { id: "3", kind: "recording", title: "Standup with Alice", subtitle: "yesterday", action: noop },
  { id: "4", kind: "memory", title: "user.company is Attune", action: noop },
];

describe("scoreFuzzy", () => {
  it("returns positive when every token is present", () => {
    expect(scoreFuzzy("library", "Open Library")).toBeGreaterThan(0);
    expect(scoreFuzzy("open lib", "Open Library")).toBeGreaterThan(0);
  });

  it("returns zero when any token is absent", () => {
    expect(scoreFuzzy("xyz", "Open Library")).toBe(0);
    expect(scoreFuzzy("open mars", "Open Library")).toBe(0);
  });

  it("scores prefix matches higher than substring matches", () => {
    const prefix = scoreFuzzy("lib", "library scan");
    const substring = scoreFuzzy("ibr", "library scan");
    expect(prefix).toBeGreaterThan(substring);
  });

  it("returns a positive sentinel for an empty query", () => {
    expect(scoreFuzzy("", "anything")).toBeGreaterThan(0);
  });
});

describe("rank", () => {
  it("returns items in input order when query is empty", () => {
    const result = rank(items, "");
    expect(result.map((i) => i.id)).toEqual(["1", "2", "3", "4"]);
  });

  it("drops zero-score items", () => {
    const result = rank(items, "mars");
    expect(result).toHaveLength(0);
  });

  it("orders by descending score", () => {
    const result = rank(items, "open");
    expect(result[0]?.id).toBe("1");
  });

  it("matches against keywords + subtitle", () => {
    expect(rank(items, "yesterday").map((i) => i.id)).toContain("3");
    expect(rank(items, "recordings").map((i) => i.id)).toContain("1");
    expect(rank(items, "meeting").map((i) => i.id)).toContain("2");
  });
});
