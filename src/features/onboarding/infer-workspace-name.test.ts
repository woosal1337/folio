import { describe, expect, it } from "vitest";
import { inferWorkspaceNameFromEmail } from "./infer-workspace-name";

describe("inferWorkspaceNameFromEmail", () => {
  it("strips a single-part TLD and capitalises", () => {
    expect(inferWorkspaceNameFromEmail("ege@clinora.ai")).toBe("Clinora");
    expect(inferWorkspaceNameFromEmail("alice@acme.io")).toBe("Acme");
    expect(inferWorkspaceNameFromEmail("bob@example.com")).toBe("Example");
  });

  it("handles two-part public suffixes", () => {
    expect(inferWorkspaceNameFromEmail("alice@acme.co.uk")).toBe("Acme");
    expect(inferWorkspaceNameFromEmail("bob@example.com.au")).toBe("Example");
    expect(inferWorkspaceNameFromEmail("carol@startup.com.tr")).toBe("Startup");
  });

  it("hyphenates into title-cased words", () => {
    expect(inferWorkspaceNameFromEmail("alice@deep-mind.com")).toBe("Deep Mind");
    expect(inferWorkspaceNameFromEmail("bob@open_ai.com")).toBe("Open Ai");
  });

  it("lowercases the input domain before parsing", () => {
    expect(inferWorkspaceNameFromEmail("alice@CLINORA.AI")).toBe("Clinora");
  });

  it("returns empty for malformed inputs", () => {
    expect(inferWorkspaceNameFromEmail("")).toBe("");
    expect(inferWorkspaceNameFromEmail("not-an-email")).toBe("");
    expect(inferWorkspaceNameFromEmail("alice@")).toBe("");
    expect(inferWorkspaceNameFromEmail("@example.com")).toBe("Example");
    expect(inferWorkspaceNameFromEmail("alice@no-tld")).toBe("");
    expect(inferWorkspaceNameFromEmail("with space@example.com")).toBe("Example");
  });
});
