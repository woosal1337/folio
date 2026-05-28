import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/shared/lib/ipc", () => ({
  recordingStatus: vi.fn(),
  startRecording: vi.fn(),
  stopRecording: vi.fn(),
  setTrayRecording: vi.fn(),
  getRecording: vi.fn(),
  runAgent: vi.fn(),
  transcribeRecording: vi.fn(),
  hasOpenAiKey: vi.fn().mockResolvedValue(false),
}));

import { recordingStatus, startRecording, stopRecording } from "@/shared/lib/ipc";
import { useRecording } from "./recording-store";

const mockedStatus = vi.mocked(recordingStatus);
const mockedStart = vi.mocked(startRecording);
const mockedStop = vi.mocked(stopRecording);

beforeEach(() => {
  // Reset the store between tests.
  useRecording.setState({
    recording: false,
    startedAt: null,
    elapsed: 0,
    channels: [],
    error: null,
    busy: false,
    lastSavedDir: null,
    _tickerId: null,
  });
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("recording-store: start", () => {
  it("transitions to recording on success and captures channels", async () => {
    mockedStart.mockResolvedValueOnce({
      recording: true,
      elapsed_secs: 0n,
      channels: ["mic", "system"],
      session_dir: "/tmp/attune/2026-05-28-10-00-00",
    });
    const { result } = renderHook(() => useRecording());
    await act(async () => {
      await result.current.start();
    });
    expect(useRecording.getState().recording).toBe(true);
    expect(useRecording.getState().channels).toEqual(["mic", "system"]);
    expect(useRecording.getState().error).toBeNull();
    expect(useRecording.getState().busy).toBe(false);
    expect(useRecording.getState().liveSessionDir).toBe(
      "/tmp/attune/2026-05-28-10-00-00"
    );
  });

  it("surfaces backend errors without transitioning", async () => {
    mockedStart.mockRejectedValueOnce(new Error("permission denied"));
    const { result } = renderHook(() => useRecording());
    await act(async () => {
      await result.current.start();
    });
    expect(useRecording.getState().recording).toBe(false);
    expect(useRecording.getState().error).toContain("permission denied");
  });
});

describe("recording-store: stop", () => {
  it("transitions to idle and records the saved dir", async () => {
    useRecording.setState({
      recording: true,
      startedAt: Date.now(),
      channels: ["mic"],
    });
    mockedStop.mockResolvedValueOnce({
      artifacts: {
        session_dir: "/tmp/2026-05-22",
        mic_path: "/tmp/2026-05-22/mic.wav",
        system_path: null,
        started_at: "2026-05-22T00:00:00Z",
        stopped_at: "2026-05-22T00:01:00Z",
      },
      label: "2026-05-22",
    });

    const { result } = renderHook(() => useRecording());
    await act(async () => {
      await result.current.stop();
    });

    expect(useRecording.getState().recording).toBe(false);
    expect(useRecording.getState().lastSavedDir).toBe("/tmp/2026-05-22");
    expect(useRecording.getState().elapsed).toBe(0);
  });
});

describe("recording-store: syncFromBackend", () => {
  it("does nothing when backend reports idle", async () => {
    mockedStatus.mockResolvedValueOnce({
      recording: false,
      elapsed_secs: 0n,
      channels: [],
      session_dir: null,
    });
    const { result } = renderHook(() => useRecording());
    await act(async () => {
      await result.current.syncFromBackend();
    });
    expect(useRecording.getState().recording).toBe(false);
  });

  it("adopts the backend's in-flight session", async () => {
    mockedStatus.mockResolvedValueOnce({
      recording: true,
      elapsed_secs: 42n,
      channels: ["mic", "system"],
      session_dir: "/tmp/attune/2026-05-28-10-00-00",
    });
    const { result } = renderHook(() => useRecording());
    await act(async () => {
      await result.current.syncFromBackend();
    });
    expect(useRecording.getState().recording).toBe(true);
    expect(useRecording.getState().elapsed).toBe(42);
    expect(useRecording.getState().channels).toEqual(["mic", "system"]);
  });
});
