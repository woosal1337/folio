import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/shared/lib/ipc", () => ({
  recordingStatus: vi.fn(),
  startRecording: vi.fn(),
  stopRecording: vi.fn(),
  pauseRecording: vi.fn(),
  resumeRecording: vi.fn(),
  setTrayRecording: vi.fn(),
  showRecordingBar: vi.fn().mockResolvedValue(undefined),
  hideRecordingBar: vi.fn().mockResolvedValue(undefined),
  getRecording: vi.fn(),
  runAgent: vi.fn(),
  transcribeRecording: vi.fn(),
  hasOpenAiKey: vi.fn().mockResolvedValue(false),
}));

import {
  recordingStatus,
  startRecording,
  stopRecording,
  pauseRecording,
  resumeRecording,
  showRecordingBar,
  hideRecordingBar,
} from "@/shared/lib/ipc";
import { useRecording } from "./recording-store";

const mockedStatus = vi.mocked(recordingStatus);
const mockedStart = vi.mocked(startRecording);
const mockedStop = vi.mocked(stopRecording);
const mockedPause = vi.mocked(pauseRecording);
const mockedResume = vi.mocked(resumeRecording);
const mockedShowBar = vi.mocked(showRecordingBar);
const mockedHideBar = vi.mocked(hideRecordingBar);

beforeEach(() => {
  // Reset the store between tests.
  useRecording.setState({
    recording: false,
    paused: false,
    startedAt: null,
    elapsed: 0,
    channels: [],
    error: null,
    busy: false,
    lastSavedDir: null,
    liveSessionDir: null,
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
      paused: false,
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
      paused: false,
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
      paused: false,
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

describe("recording-store: pause/resume (GET-149)", () => {
  it("pause freezes the timer and marks the note paused", async () => {
    mockedPause.mockResolvedValueOnce({
      recording: false,
      paused: true,
      elapsed_secs: 30n,
      channels: [],
      session_dir: "/tmp/attune/note",
    });
    useRecording.setState({
      recording: true,
      paused: false,
      startedAt: Date.now() - 30_000,
      elapsed: 30,
      channels: ["mic"],
    });
    const { result } = renderHook(() => useRecording());
    await act(async () => {
      await result.current.pause();
    });
    const s = useRecording.getState();
    expect(s.recording).toBe(false);
    expect(s.paused).toBe(true);
    expect(s.elapsed).toBe(30);
    expect(s.liveSessionDir).toBe("/tmp/attune/note");
  });

  it("resume continues recording with continuous elapsed", async () => {
    mockedResume.mockResolvedValueOnce({
      recording: true,
      paused: false,
      elapsed_secs: 30n,
      channels: ["mic", "system"],
      session_dir: "/tmp/attune/note",
    });
    // Resume only acts on a paused note (matches the real precondition —
    // and the store guard that ignores a stray resume otherwise).
    useRecording.setState({ recording: false, paused: true, elapsed: 30 });
    const { result } = renderHook(() => useRecording());
    await act(async () => {
      await result.current.resume();
    });
    const s = useRecording.getState();
    expect(s.recording).toBe(true);
    expect(s.paused).toBe(false);
    expect(s.elapsed).toBe(30);
    expect(s.channels).toEqual(["mic", "system"]);
  });
});

describe("recording-store: pause/resume/stop guards (re-entrancy + state)", () => {
  it("pause is a no-op when not recording", async () => {
    const { result } = renderHook(() => useRecording());
    await act(async () => {
      await result.current.pause();
    });
    expect(mockedPause).not.toHaveBeenCalled();
    expect(useRecording.getState().error).toBeNull();
  });

  it("resume is a no-op when not paused", async () => {
    const { result } = renderHook(() => useRecording());
    await act(async () => {
      await result.current.resume();
    });
    expect(mockedResume).not.toHaveBeenCalled();
    expect(useRecording.getState().error).toBeNull();
  });

  it("stop is a no-op when idle", async () => {
    const { result } = renderHook(() => useRecording());
    await act(async () => {
      await result.current.stop();
    });
    expect(mockedStop).not.toHaveBeenCalled();
  });

  it("a second pause while one is in flight is ignored (no double-fire)", async () => {
    useRecording.setState({ recording: true, paused: false });
    mockedPause.mockResolvedValue({
      recording: false,
      elapsed_secs: 5n,
      channels: [],
      session_dir: "/tmp/attune/note",
      paused: true,
    });
    const { result } = renderHook(() => useRecording());
    await act(async () => {
      // Fire two pauses without awaiting the first — the busy guard must
      // drop the second so the backend pause runs exactly once.
      await Promise.all([result.current.pause(), result.current.pause()]);
    });
    expect(mockedPause).toHaveBeenCalledTimes(1);
  });
});

describe("recording-store: floating bar lifecycle", () => {
  it("shows the bar on start and hides it on stop", async () => {
    mockedStart.mockResolvedValueOnce({
      recording: true,
      elapsed_secs: 0n,
      channels: ["mic"],
      session_dir: "/tmp/attune/note",
      paused: false,
    });
    mockedStop.mockResolvedValueOnce({
      artifacts: { session_dir: "/tmp/attune/note" },
    } as never);
    const { result } = renderHook(() => useRecording());
    await act(async () => {
      await result.current.start();
    });
    expect(mockedShowBar).toHaveBeenCalledTimes(1);
    await act(async () => {
      await result.current.stop();
    });
    expect(mockedHideBar).toHaveBeenCalledTimes(1);
  });

  it("re-shows the bar on resume (stays up across pause/resume)", async () => {
    useRecording.setState({ recording: false, paused: true, elapsed: 10 });
    mockedResume.mockResolvedValueOnce({
      recording: true,
      elapsed_secs: 10n,
      channels: ["mic"],
      session_dir: "/tmp/attune/note",
      paused: false,
    });
    const { result } = renderHook(() => useRecording());
    await act(async () => {
      await result.current.resume();
    });
    expect(mockedShowBar).toHaveBeenCalled();
    expect(mockedHideBar).not.toHaveBeenCalled();
  });

  it("tears the UI down instantly on stop, before the backend finalize resolves", async () => {
    useRecording.setState({ recording: true, paused: false, elapsed: 5 });
    let resolveStop!: (v: unknown) => void;
    mockedStop.mockImplementationOnce(
      () =>
        new Promise((res) => {
          resolveStop = res as (v: unknown) => void;
        }) as never
    );
    // Kick off stop but don't await — the optimistic teardown is
    // synchronous and must have already run while ipcStop is still pending.
    const stopPromise = useRecording.getState().stop();
    expect(useRecording.getState().recording).toBe(false);
    expect(useRecording.getState().paused).toBe(false);
    expect(mockedHideBar).toHaveBeenCalledTimes(1);
    // Now let the (slow) finalize complete and confirm the saved dir lands.
    await act(async () => {
      resolveStop({ artifacts: { session_dir: "/tmp/attune/note" } });
      await stopPromise;
    });
    expect(useRecording.getState().lastSavedDir).toBe("/tmp/attune/note");
  });
});
