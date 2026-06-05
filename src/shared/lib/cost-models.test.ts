import { describe, expect, it } from "vitest";

import { estimateChatCompletionCost } from "./cost-models";

describe("estimateChatCompletionCost", () => {
  it("returns 0 when usage is missing", () => {
    expect(
      estimateChatCompletionCost({
        model: "gpt-4o-mini",
        promptTokens: null,
        completionTokens: null,
      })
    ).toBe(0);
  });

  it("prices a typical gpt-4o-mini run", () => {
    const usd = estimateChatCompletionCost({
      model: "gpt-4o-mini",
      promptTokens: 1_000,
      completionTokens: 250,
    });
    expect(usd).toBeCloseTo(0.0003, 6);
  });

  it("collapses dated variants to the base sku", () => {
    const dated = estimateChatCompletionCost({
      model: "gpt-4o-2024-08-06",
      promptTokens: 1_000,
      completionTokens: 1_000,
    });
    const base = estimateChatCompletionCost({
      model: "gpt-4o",
      promptTokens: 1_000,
      completionTokens: 1_000,
    });
    expect(dated).toBeCloseTo(base, 6);
  });

  it("falls back to a conservative rate for unknown models", () => {
    const usd = estimateChatCompletionCost({
      model: "some-weird-future-model",
      promptTokens: 1_000_000,
      completionTokens: 0,
    });

    expect(usd).toBeCloseTo(1.0, 4);
  });
});
