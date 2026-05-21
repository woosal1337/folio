/* Mirror of Rust data models exposed via Tauri commands. Keep field names
 * in sync with the Rust side. Consider auto-generation via specta in a
 * future iteration. */

export interface DeviceInfo {
  name: string;
  is_default: boolean;
  default_sample_rate: number | null;
  default_channels: number | null;
}

export interface Settings {
  mic_device: string | null;
  system_audio_enabled: boolean;
  output_dir: string;
  notes_dir: string;
  tasks_path: string;
  transcripts_dir: string;
  theme: "light" | "dark";
  transcriber: "openai" | "local_whisper";
  openai_api_key: string;
  transcription_language: string;
  dictionary_terms: string[];
}

export type Theme = "light" | "dark";

export interface RecordingSummary {
  session_dir: string;
  label: string;
  duration_seconds: number;
  mic_bytes: number | null;
  system_bytes: number | null;
  mic_sample_rate: number | null;
  system_sample_rate: number | null;
}

export interface TranscriptSegment {
  start_ms: number;
  end_ms: number;
  text: string;
  speaker: string | null;
  language: string | null;
}

export interface Transcript {
  id: string;
  session_dir: string;
  recording_label: string;
  created_at: string;
  provider: string;
  model: string;
  language: string | null;
  duration_seconds: number;
  segments: TranscriptSegment[];
}
