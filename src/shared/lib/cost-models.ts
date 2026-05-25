/**
 * Per-million-token pricing for chat-completion models Attune calls.
 * Numbers are USD as of 2026-05; the user can override / adjust by
 * editing this file. Unknown models fall back to a conservative
 * gpt-4o-mini-equivalent rate so the UI never returns NaN — the
 * estimate just runs a touch high.
 *
 * v2 roadmap finding 090 / GET-83.
 */

interface ModelPrice {
  /** USD per 1 000 000 input tokens. */
  inputPerMTokens: number;
  /** USD per 1 000 000 output tokens. */
  outputPerMTokens: number;
}

const PRICING: Record<string, ModelPrice> = {
  "gpt-4o": { inputPerMTokens: 2.5, outputPerMTokens: 10.0 },
  "gpt-4o-mini": { inputPerMTokens: 0.15, outputPerMTokens: 0.6 },
  "gpt-4-turbo": { inputPerMTokens: 10.0, outputPerMTokens: 30.0 },
  "gpt-4.1": { inputPerMTokens: 2.0, outputPerMTokens: 8.0 },
  "gpt-4.1-mini": { inputPerMTokens: 0.4, outputPerMTokens: 1.6 },
  "gpt-5": { inputPerMTokens: 5.0, outputPerMTokens: 20.0 },
  "gpt-5-mini": { inputPerMTokens: 0.5, outputPerMTokens: 2.0 },
  o1: { inputPerMTokens: 15.0, outputPerMTokens: 60.0 },
  "o1-mini": { inputPerMTokens: 3.0, outputPerMTokens: 12.0 },
};

/** Conservative fallback when we don't recognise the model id. */
const FALLBACK: ModelPrice = { inputPerMTokens: 1.0, outputPerMTokens: 4.0 };

/**
 * Estimate the USD cost of a single chat-completion run given the
 * model id and the prompt + completion token counts the provider
 * reported. Returns 0 when either count is missing (the model could
 * not report usage — common with the Local Whisper path which has
 * no LLM cost anyway).
 */
export function estimateChatCompletionCost(args: {
  model: string;
  promptTokens: number | null | undefined;
  completionTokens: number | null | undefined;
}): number {
  const pt = args.promptTokens ?? 0;
  const ct = args.completionTokens ?? 0;
  if (pt === 0 && ct === 0) return 0;
  const norm = normalizeModelId(args.model);
  const price = PRICING[norm] ?? FALLBACK;
  return (
    (pt * price.inputPerMTokens) / 1_000_000 + (ct * price.outputPerMTokens) / 1_000_000
  );
}

/** Strip the date/version suffix some providers append (`gpt-4o-2024-08-06`). */
function normalizeModelId(model: string): string {
  const lower = model.toLowerCase().trim();
  // Try the exact id first; fall through to a prefix match against
  // the keys of PRICING (longest first) so date-stamped variants
  // collapse to their base sku.
  if (PRICING[lower]) return lower;
  const keys = Object.keys(PRICING).sort((a, b) => b.length - a.length);
  for (const k of keys) {
    if (lower.startsWith(k)) return k;
  }
  return lower;
}
