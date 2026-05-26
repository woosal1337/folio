import { describe, expect, it } from "vitest";

import { parseAutoname } from "./autoname";

describe("parseAutoname", () => {
  it("extracts title, tags, and subtitle from the canonical JSON shape", () => {
    const parsed = parseAutoname(
      '{"title":"Testing New Recording Features","tags":["recording","video","music"],"subtitle":"Discussion on testing video and audio recording features."}'
    );
    expect(parsed).toEqual({
      title: "Testing New Recording Features",
      tags: ["recording", "video", "music"],
      subtitle: "Discussion on testing video and audio recording features.",
    });
  });

  it("tolerates newlines and whitespace inside the JSON", () => {
    const parsed = parseAutoname(
      '{\n"title": "Demo",\n"tags": ["a", "b"],\n"subtitle": "ok"\n}'
    );
    expect(parsed?.title).toBe("Demo");
    expect(parsed?.tags).toEqual(["a", "b"]);
  });

  it("lifts the JSON block out of surrounding prose / markdown fences", () => {
    const parsed = parseAutoname(
      'Here is the result:\n```json\n{"title":"X","tags":["y"],"subtitle":"z"}\n```'
    );
    expect(parsed).toEqual({ title: "X", tags: ["y"], subtitle: "z" });
  });

  it("returns the empty-but-valid shape the prompt asks for on noisy input", () => {
    const parsed = parseAutoname('{"title":"","tags":[],"subtitle":""}');
    expect(parsed).toEqual({ title: "", tags: [], subtitle: "" });
  });

  it("strips non-string tags defensively", () => {
    const parsed = parseAutoname('{"title":"T","tags":["ok",42,null,""],"subtitle":""}');
    expect(parsed?.tags).toEqual(["ok"]);
  });

  it("returns null when there is no JSON object at all", () => {
    expect(parseAutoname("Sorry, I cannot title this transcript.")).toBeNull();
  });

  it("returns null when the JSON is malformed", () => {
    expect(parseAutoname('{"title": "broken",')).toBeNull();
  });
});
