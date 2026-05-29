import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const h = vi.hoisted(() => {
  const start = vi.fn();
  const storeState = { recording: false, busy: false, start };
  return { createNote: vi.fn(), navigate: vi.fn(), start, storeState };
});

vi.mock("react-router-dom", () => ({ useNavigate: () => h.navigate }));
vi.mock("@/shared/lib/ipc", () => ({ createNote: h.createNote }));
vi.mock("@/shared/stores/recording-store", () => {
  const useRecording = (sel?: (s: typeof h.storeState) => unknown) =>
    sel ? sel(h.storeState) : h.storeState;
  (useRecording as unknown as { getState: () => typeof h.storeState }).getState = () =>
    h.storeState;
  return { useRecording };
});

import { useQuickNote, useTakeNotes } from "./use-take-notes";

const NOTE = { label: "2026-05-29-note", session_dir: "/tmp/Attune/2026-05-29-note" };

beforeEach(() => {
  vi.clearAllMocks();
  h.storeState.recording = false;
  h.storeState.busy = false;
  h.createNote.mockResolvedValue(NOTE);
});

describe("useQuickNote", () => {
  it("creates an empty note and opens it without recording", async () => {
    const { result } = renderHook(() => useQuickNote());
    result.current();
    await waitFor(() => expect(h.navigate).toHaveBeenCalled());
    expect(h.createNote).toHaveBeenCalledTimes(1);
    expect(h.navigate).toHaveBeenCalledWith(`/editor/${NOTE.label}`, {
      state: { recording: NOTE },
    });
    expect(h.start).not.toHaveBeenCalled();
  });
});

describe("useTakeNotes", () => {
  it("creates a note, opens it, and records into its dir", async () => {
    const { result } = renderHook(() => useTakeNotes());
    result.current();
    await waitFor(() => expect(h.start).toHaveBeenCalled());
    expect(h.createNote).toHaveBeenCalledTimes(1);
    expect(h.navigate).toHaveBeenCalledWith(`/editor/${NOTE.label}`, {
      state: { recording: NOTE },
    });
    expect(h.start).toHaveBeenCalledWith(NOTE.session_dir);
  });

  it("does not start a second recording when one is already active", async () => {
    h.storeState.recording = true;
    const { result } = renderHook(() => useTakeNotes());
    result.current();
    await waitFor(() => expect(h.navigate).toHaveBeenCalled());
    expect(h.start).not.toHaveBeenCalled();
  });
});
