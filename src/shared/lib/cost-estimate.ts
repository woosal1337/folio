/**
 * Cost estimate for the OpenAI Whisper transcription path.
 *
 * Whisper pricing: $0.006 / minute as of 2026-05. We round up to the
 * nearest second to match how OpenAI bills, then multiply.
 *
 * Threshold for surfacing the confirm modal is whichever comes first:
 * - WAV payload exceeds CONFIRM_THRESHOLD_BYTES (default 50MB)
 * - Estimated cost exceeds CONFIRM_THRESHOLD_USD (default $0.25)
 *
 * Below both thresholds we don't bother the user — uploading a 10
 * minute meeting is $0.06 and no one needs a modal for that. Above,
 * we block on explicit confirmation.
 *
 * v2 roadmap finding 055.
 */

export const WHISPER_USD_PER_MINUTE = 0.006;
export const CONFIRM_THRESHOLD_BYTES = 50 * 1024 * 1024;
export const CONFIRM_THRESHOLD_USD = 0.25;

export interface CostEstimate {
  /** Total WAV bytes that will be uploaded (mic + system channels). */
  totalBytes: number;
  /** Recording duration in minutes (rounded up to the nearest second). */
  durationMinutes: number;
  /** Estimated cost in USD at Whisper's posted price. */
  estimatedUsd: number;
  /** True when either threshold is exceeded — the UI should confirm. */
  exceedsThreshold: boolean;
}

export function estimateOpenAITranscribeCost(args: {
  durationSeconds: number;
  micBytes: number;
  systemBytes: number;
}): CostEstimate {
  const totalBytes = (args.micBytes ?? 0) + (args.systemBytes ?? 0);
  // Bill per second, rounded up.
  const billedSeconds = Math.ceil(Math.max(0, args.durationSeconds));
  const durationMinutes = billedSeconds / 60;
  const estimatedUsd = durationMinutes * WHISPER_USD_PER_MINUTE;
  const exceedsThreshold =
    totalBytes > CONFIRM_THRESHOLD_BYTES || estimatedUsd > CONFIRM_THRESHOLD_USD;
  return { totalBytes, durationMinutes, estimatedUsd, exceedsThreshold };
}

/** Format bytes as "1.4 GB" / "47.2 MB" / "812 KB". */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

/** Format USD with $0.00 (3 decimals when under $1). */
export function formatUsd(usd: number): string {
  if (usd < 1) return `$${usd.toFixed(3)}`;
  return `$${usd.toFixed(2)}`;
}

/** Format duration as "1h 23m" / "42m" / "47s". */
export function formatDuration(minutes: number): string {
  const totalSeconds = Math.round(minutes * 60);
  if (totalSeconds < 60) return `${totalSeconds}s`;
  const m = Math.floor(totalSeconds / 60);
  const s = totalSeconds % 60;
  if (m < 60) return s > 0 ? `${m}m ${s}s` : `${m}m`;
  const h = Math.floor(m / 60);
  const mm = m % 60;
  return mm > 0 ? `${h}h ${mm}m` : `${h}h`;
}
