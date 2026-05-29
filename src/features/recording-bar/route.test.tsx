import { render, screen, waitFor, fireEvent, cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@/shared/lib/ipc", () => ({
  recordingStatus: vi.fn(),
  recordingBarStop: vi.fn().mockResolvedValue(undefined),
  recordingBarPause: vi.fn().mockResolvedValue(undefined),
  recordingBarResume: vi.fn().mockResolvedValue(undefined),
  hideRecordingBar: vi.fn().mockResolvedValue(undefined),
  startWindowDrag: vi.fn().mockResolvedValue(undefined),
}));

import {
  recordingStatus,
  recordingBarStop,
  recordingBarPause,
  recordingBarResume,
} from "@/shared/lib/ipc";
import RecordingBar from "./route";

const mockedStatus = vi.mocked(recordingStatus);

const status = (over: Partial<Awaited<ReturnType<typeof recordingStatus>>> = {}) => ({
  recording: true,
  elapsed_secs: 12n,
  channels: ["mic"],
  session_dir: "/tmp/attune/note",
  paused: false,
  ...over,
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("RecordingBar widget", () => {
  it("renders the elapsed time and a recording indicator", async () => {
    mockedStatus.mockResolvedValue(status());
    render(<RecordingBar />);
    await waitFor(() => expect(screen.getByText("0:12")).toBeInTheDocument());
    expect(screen.getByTitle("Recording")).toBeInTheDocument();
    // Recording (not paused) → Pause control + Stop control present.
    expect(
      screen.getByRole("button", { name: /pause recording/i })
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /stop recording/i })).toBeInTheDocument();
  });

  it("shows the paused state with a resume control", async () => {
    mockedStatus.mockResolvedValue(status({ recording: false, paused: true }));
    render(<RecordingBar />);
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: /resume recording/i })
      ).toBeInTheDocument()
    );
    expect(screen.getByTitle("Paused")).toBeInTheDocument();
  });

  it("Stop routes through recording_bar_stop", async () => {
    mockedStatus.mockResolvedValue(status());
    render(<RecordingBar />);
    const stop = await screen.findByRole("button", { name: /stop recording/i });
    fireEvent.click(stop);
    expect(recordingBarStop).toHaveBeenCalledTimes(1);
  });

  it("Pause routes through recording_bar_pause when recording", async () => {
    mockedStatus.mockResolvedValue(status());
    render(<RecordingBar />);
    const pause = await screen.findByRole("button", { name: /pause recording/i });
    fireEvent.click(pause);
    expect(recordingBarPause).toHaveBeenCalledTimes(1);
    expect(recordingBarResume).not.toHaveBeenCalled();
  });

  it("Resume routes through recording_bar_resume when paused", async () => {
    mockedStatus.mockResolvedValue(status({ recording: false, paused: true }));
    render(<RecordingBar />);
    const resume = await screen.findByRole("button", { name: /resume recording/i });
    fireEvent.click(resume);
    expect(recordingBarResume).toHaveBeenCalledTimes(1);
    expect(recordingBarPause).not.toHaveBeenCalled();
  });
});
