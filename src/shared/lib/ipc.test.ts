import { afterEach, describe, expect, it, vi } from "vitest";

import { IpcError } from "./ipc";

// Mock the Tauri invoke before the IPC module loads.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
const mockedInvoke = vi.mocked(invoke);

afterEach(() => {
  mockedInvoke.mockReset();
});

describe("IpcError", () => {
  it("formats a string cause", () => {
    const err = new IpcError("foo", "boom");
    expect(err.message).toBe("ipc foo failed: boom");
    expect(err.command).toBe("foo");
    expect(err.cause).toBe("boom");
  });

  it("extracts message from Error instances", () => {
    const cause = new Error("nested");
    const err = new IpcError("bar", cause);
    expect(err.message).toBe("ipc bar failed: nested");
  });

  it("falls back to JSON for unknown shapes", () => {
    const err = new IpcError("baz", { code: 42 });
    expect(err.message).toBe('ipc baz failed: {"code":42}');
  });
});

describe("ipc wrappers", () => {
  it("wraps thrown failures in IpcError tagged with the command", async () => {
    mockedInvoke.mockRejectedValueOnce("not recording");
    const { stopRecording } = await import("./ipc");
    try {
      await stopRecording();
      throw new Error("expected stopRecording to throw");
    } catch (e) {
      expect(e).toBeInstanceOf(IpcError);
      expect((e as IpcError).command).toBe("stop_recording");
      expect((e as IpcError).message).toContain("not recording");
    }
  });

  it("returns the typed result on success", async () => {
    mockedInvoke.mockResolvedValueOnce("pong, ada");
    const { ping } = await import("./ipc");
    await expect(ping("ada")).resolves.toBe("pong, ada");
    expect(mockedInvoke).toHaveBeenCalledWith("ping", { name: "ada" });
  });
});
