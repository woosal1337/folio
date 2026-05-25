/**
 * Cross-component coordination for "jump the right audio player to N
 * seconds". Decouples the transcript editor (which knows the timestamp
 * the user clicked) from the audio players higher up in the editor
 * tree, without prop-drilling refs through unrelated layers.
 *
 * Both transcript segments and audio players carry a `channel` id
 * ("mic" / "system"); the player whose label matches the channel is
 * the one that seeks and plays.
 *
 * v2 roadmap finding 102 (GET-114).
 */

export const SEEK_AUDIO_EVENT = "attune:seek-audio";

export interface SeekAudioDetail {
  /** "mic" | "system" — the audio channel the click came from. */
  channel: string;
  /** Absolute position into the audio file, in seconds. */
  seconds: number;
}

export function dispatchSeekAudio(detail: SeekAudioDetail): void {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent<SeekAudioDetail>(SEEK_AUDIO_EVENT, { detail }));
}

/** Subscribe to seek-audio events. Returns the unsubscribe function. */
export function onSeekAudio(handler: (detail: SeekAudioDetail) => void): () => void {
  const listener = (e: Event) => {
    const ce = e as CustomEvent<SeekAudioDetail>;
    if (ce.detail) handler(ce.detail);
  };
  window.addEventListener(SEEK_AUDIO_EVENT, listener);
  return () => window.removeEventListener(SEEK_AUDIO_EVENT, listener);
}
