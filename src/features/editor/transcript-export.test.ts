import { describe, expect, it } from "vitest";

import {
  renderTranscript,
  segmentMatches,
  srtTimestamp,
  toSrt,
  toTxt,
  toVtt,
  txtTimestamp,
  vttTimestamp,
} from "./transcript-export";
import type { SessionTranscript } from "@/shared/types/SessionTranscript";

const sample: SessionTranscript = {
  channels: [
    {
      channel: "mic",
      language: "en",
      segments: [
        { start_seconds: 0, end_seconds: 1.25, text: "Hello there." },
        { start_seconds: 3.5, end_seconds: 5, text: "How are you?" },
      ],
    },
    {
      channel: "system",
      language: "en",
      segments: [
        { start_seconds: 1.5, end_seconds: 3, text: "Hi, doing well." },
        { start_seconds: 5.25, end_seconds: 7, text: "  " }, // blank → dropped
      ],
    },
  ],
};

describe("srtTimestamp", () => {
  it("pads the timestamp to HH:MM:SS,mmm", () => {
    expect(srtTimestamp(0)).toBe("00:00:00,000");
    expect(srtTimestamp(1.25)).toBe("00:00:01,250");
    expect(srtTimestamp(61.001)).toBe("00:01:01,001");
    expect(srtTimestamp(3725.5)).toBe("01:02:05,500");
  });

  it("clamps negatives and NaN to zero", () => {
    expect(srtTimestamp(-5)).toBe("00:00:00,000");
    expect(srtTimestamp(Number.NaN)).toBe("00:00:00,000");
  });
});

describe("vttTimestamp", () => {
  it("uses a dot separator", () => {
    expect(vttTimestamp(1.25)).toBe("00:00:01.250");
  });
});

describe("txtTimestamp", () => {
  it("wraps the VTT stamp in square brackets", () => {
    expect(txtTimestamp(1.25)).toBe("[00:00:01.250]");
  });
});

describe("toSrt", () => {
  it("renders cues sorted by start time, numbered, with speaker prefix", () => {
    const out = toSrt(sample);
    expect(out).toBe(
      [
        "1",
        "00:00:00,000 --> 00:00:01,250",
        "You: Hello there.",
        "",
        "2",
        "00:00:01,500 --> 00:00:03,000",
        "Others: Hi, doing well.",
        "",
        "3",
        "00:00:03,500 --> 00:00:05,000",
        "You: How are you?",
        "",
      ].join("\n")
    );
  });

  it("drops blank/whitespace-only segments", () => {
    expect(toSrt(sample)).not.toContain("Others:\n");
  });
});

describe("toVtt", () => {
  it("starts with the WEBVTT magic header and uses <v Speaker> tags", () => {
    const out = toVtt(sample);
    expect(out.startsWith("WEBVTT\n\n")).toBe(true);
    expect(out).toContain("<v You>Hello there.");
    expect(out).toContain("<v Others>Hi, doing well.");
  });

  it("uses dot timestamp separators", () => {
    expect(toVtt(sample)).toContain("00:00:00.000 --> 00:00:01.250");
  });
});

describe("toTxt", () => {
  it("emits one line per cue, prefixed with [stamp] Speaker:", () => {
    const out = toTxt(sample);
    expect(out).toBe(
      [
        "[00:00:00.000] You: Hello there.",
        "[00:00:01.500] Others: Hi, doing well.",
        "[00:00:03.500] You: How are you?",
      ].join("\n")
    );
  });
});

describe("renderTranscript", () => {
  it("dispatches on format", () => {
    expect(renderTranscript(sample, "srt").startsWith("1\n")).toBe(true);
    expect(renderTranscript(sample, "vtt").startsWith("WEBVTT")).toBe(true);
    expect(renderTranscript(sample, "txt").startsWith("[")).toBe(true);
  });
});

describe("segmentMatches", () => {
  it("returns true when the query is blank", () => {
    expect(segmentMatches({ start_seconds: 0, end_seconds: 1, text: "abc" }, "")).toBe(
      true
    );
    expect(
      segmentMatches({ start_seconds: 0, end_seconds: 1, text: "abc" }, "   ")
    ).toBe(true);
  });

  it("is case-insensitive substring match", () => {
    expect(
      segmentMatches(
        { start_seconds: 0, end_seconds: 1, text: "Hello THERE." },
        "there"
      )
    ).toBe(true);
    expect(
      segmentMatches({ start_seconds: 0, end_seconds: 1, text: "Hello there." }, "xyz")
    ).toBe(false);
  });
});
