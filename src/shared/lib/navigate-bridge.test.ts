import { beforeEach, describe, expect, it, vi } from "vitest";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let mod: any;

beforeEach(async () => {
  vi.resetModules();
  mod = await import("./navigate-bridge");
});

describe("navigate-bridge", () => {
  it("queues navigations before register, replays on register", () => {
    const calls: string[] = [];
    mod.bridgeNavigate("/library");
    mod.bridgeNavigate("/tasks");
    mod.bridgeNavigate("/memory");
    expect(calls).toHaveLength(0);

    mod.registerNavigateFn(((to: string) => calls.push(to)) as never);
    expect(calls).toEqual(["/library", "/tasks", "/memory"]);
  });

  it("forwards immediately when already registered", () => {
    const calls: string[] = [];
    mod.registerNavigateFn(((to: string) => calls.push(to)) as never);
    mod.bridgeNavigate("/chat");
    expect(calls).toEqual(["/chat"]);
  });

  it("does not double-replay queue after second register", () => {
    const first: string[] = [];
    const second: string[] = [];

    mod.bridgeNavigate("/a");
    mod.registerNavigateFn(((to: string) => first.push(to)) as never);
    expect(first).toEqual(["/a"]);

    mod.registerNavigateFn(((to: string) => second.push(to)) as never);
    expect(second).toHaveLength(0);

    mod.bridgeNavigate("/b");
    expect(second).toEqual(["/b"]);
    expect(first).toHaveLength(1);
  });

  it("assertInternalPath accepts /‑prefixed paths", () => {
    expect(() => mod.assertInternalPath("/")).not.toThrow();
    expect(() => mod.assertInternalPath("/library")).not.toThrow();
    expect(() => mod.assertInternalPath("/editor/2026-01-01")).not.toThrow();
  });

  it("assertInternalPath rejects non-/ strings", () => {
    expect(() => mod.assertInternalPath("https://evil.example.com")).toThrow();
    expect(() => mod.assertInternalPath("folio://open")).toThrow();
    expect(() => mod.assertInternalPath("javascript:alert(1)")).toThrow();
    expect(() => mod.assertInternalPath("library")).toThrow();
  });
});
