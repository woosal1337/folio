interface ModelPrice {
  inputPerMTokens: number;

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

const FALLBACK: ModelPrice = { inputPerMTokens: 1.0, outputPerMTokens: 4.0 };

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

function normalizeModelId(model: string): string {
  const lower = model.toLowerCase().trim();

  if (PRICING[lower]) return lower;
  const keys = Object.keys(PRICING).sort((a, b) => b.length - a.length);
  for (const k of keys) {
    if (lower.startsWith(k)) return k;
  }
  return lower;
}
