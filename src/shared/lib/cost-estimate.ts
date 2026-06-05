export const WHISPER_USD_PER_MINUTE = 0.006;
export const CONFIRM_THRESHOLD_BYTES = 50 * 1024 * 1024;
export const CONFIRM_THRESHOLD_USD = 0.25;

export interface CostEstimate {
  totalBytes: number;

  durationMinutes: number;

  estimatedUsd: number;

  exceedsThreshold: boolean;
}

export function estimateOpenAITranscribeCost(args: {
  durationSeconds: number;
  micBytes: number;
  systemBytes: number;
}): CostEstimate {
  const totalBytes = (args.micBytes ?? 0) + (args.systemBytes ?? 0);

  const billedSeconds = Math.ceil(Math.max(0, args.durationSeconds));
  const durationMinutes = billedSeconds / 60;
  const estimatedUsd = durationMinutes * WHISPER_USD_PER_MINUTE;
  const exceedsThreshold =
    totalBytes > CONFIRM_THRESHOLD_BYTES || estimatedUsd > CONFIRM_THRESHOLD_USD;
  return { totalBytes, durationMinutes, estimatedUsd, exceedsThreshold };
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

export function formatUsd(usd: number): string {
  if (usd < 1) return `$${usd.toFixed(3)}`;
  return `$${usd.toFixed(2)}`;
}

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
