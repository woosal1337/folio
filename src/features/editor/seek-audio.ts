export const SEEK_AUDIO_EVENT = "attune:seek-audio";

export interface SeekAudioDetail {
  channel: string;

  seconds: number;
}

export function dispatchSeekAudio(detail: SeekAudioDetail): void {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent<SeekAudioDetail>(SEEK_AUDIO_EVENT, { detail }));
}

export function onSeekAudio(handler: (detail: SeekAudioDetail) => void): () => void {
  const listener = (e: Event) => {
    const ce = e as CustomEvent<SeekAudioDetail>;
    if (ce.detail) handler(ce.detail);
  };
  window.addEventListener(SEEK_AUDIO_EVENT, listener);
  return () => window.removeEventListener(SEEK_AUDIO_EVENT, listener);
}
