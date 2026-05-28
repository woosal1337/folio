#!/usr/bin/env node
/**
 * Voice-fixture generator for the e2e suite.
 *
 * Generates a small bank of TTS audio clips via the ElevenLabs API
 * (multi-language, multi-context) into `e2e/fixtures/audio/`. The
 * filename is deterministic per fixture id; if the file already
 * exists on disk, the entry is skipped — running this script twice
 * never burns extra ElevenLabs credits.
 *
 * Usage:
 *
 *   # Put ELEVENLABS_API_KEY in .env.e2e (gitignored) then:
 *   bun run e2e:fixtures
 *
 * The fixture catalogue + voice picks live in CATALOG below. Add new
 * entries with stable ids; the script will only generate the new
 * ones on the next run.
 */

import { promises as fs } from "node:fs";
import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "..");
const OUTPUT_DIR = path.join(REPO_ROOT, "e2e", "fixtures", "audio");
const MANIFEST_PATH = path.join(OUTPUT_DIR, "manifest.json");

// ElevenLabs default voices. These IDs are stable across the API
// surface; if any are deprecated the script will fail loudly and
// you can swap to a still-supported voice from
// https://elevenlabs.io/docs/api-reference/get-voices.
const VOICES = {
  rachel: "21m00Tcm4TlvDq8ikWAM",   // American English, calm — narration default
  domi: "AZnzlk1XvdvUeBnXmlld",     // American English, strong
  bella: "EXAVITQu4vr4xnSDxMaL",    // American English, soft
  adam: "pNInz6obpgDQGcFmaJgB",     // American English, deep
  antoni: "ErXwobaYiN019PkySvjV",   // American English, well-rounded
};

/**
 * Each fixture is rendered to `<id>.mp3`. The text is short — long
 * enough to read like a meeting snippet but short enough to keep
 * the files <100 KB on disk so they live inside the repo cache
 * without bloat. (Note: still gitignored; we cache them per dev
 * machine.)
 *
 * `model_id`: ElevenLabs supports `eleven_multilingual_v2` (best
 * for non-English) and `eleven_turbo_v2_5` (fast, mostly English).
 * We pick per-fixture so multi-language clips sound natural.
 */
const CATALOG = [
  {
    id: "en-business-1min",
    voice: VOICES.rachel,
    model_id: "eleven_multilingual_v2",
    language: "en",
    context: "business",
    text:
      "Alright, let's start. Today we need to lock the launch date and align on the GTM plan. " +
      "Marketing has the press kit ready, engineering says the migration script will land on Friday, " +
      "and customer success has briefed the top ten accounts. The biggest open question is whether " +
      "we ship the referral program at the same time, or hold it until next sprint.",
  },
  {
    id: "en-action-items",
    voice: VOICES.domi,
    model_id: "eleven_multilingual_v2",
    language: "en",
    context: "action_items",
    text:
      "Three action items. First, Mira drafts the announcement post by end of day Wednesday. " +
      "Second, Tony confirms the dashboard copy with design by Thursday morning. " +
      "Third, I'll set up the press embargo and circulate the embargoed link Friday at noon.",
  },
  {
    id: "en-clinical-consult",
    voice: VOICES.bella,
    model_id: "eleven_multilingual_v2",
    language: "en",
    context: "clinical",
    text:
      "So how have you been sleeping since we last met? You mentioned the nightmares were less " +
      "frequent. Has that continued? And the medication, any side effects you've noticed " +
      "— headaches, nausea, changes in appetite?",
  },
  {
    id: "en-product-review",
    voice: VOICES.adam,
    model_id: "eleven_multilingual_v2",
    language: "en",
    context: "product",
    text:
      "On the recording surface, the only metric that's moved is time-to-first-transcript. We're at " +
      "twenty-eight seconds median, down from forty-one. The next lever is the model-warm cache, " +
      "but that's a two-week investment.",
  },
  {
    id: "en-decision-record",
    voice: VOICES.antoni,
    model_id: "eleven_multilingual_v2",
    language: "en",
    context: "decision",
    text:
      "Decision: we are going to ship the local Whisper option as the default for the macOS build " +
      "and route OpenAI as an opt-in. The trade-off is slower first-run on older Intel Macs, but " +
      "the privacy story is much cleaner.",
  },
  {
    id: "tr-meeting",
    voice: VOICES.rachel,
    model_id: "eleven_multilingual_v2",
    language: "tr",
    context: "business",
    text:
      "Merhaba, bugünkü toplantımıza hoş geldiniz. Önce geçen haftaki kararları gözden geçirelim, " +
      "sonra önümüzdeki sprint için öncelikleri belirleyelim. Acil bir konu var mı?",
  },
  {
    id: "de-standup",
    voice: VOICES.domi,
    model_id: "eleven_multilingual_v2",
    language: "de",
    context: "standup",
    text:
      "Guten Morgen. Was haben wir gestern erledigt? Was steht heute an? Und gibt es irgendwelche " +
      "Blocker, die wir besprechen müssen?",
  },
  {
    id: "fr-product-pitch",
    voice: VOICES.bella,
    model_id: "eleven_multilingual_v2",
    language: "fr",
    context: "pitch",
    text:
      "Notre produit garde toutes les transcriptions sur votre Mac. Pas de cloud, pas de bots, " +
      "pas de comptes obligatoires pour commencer.",
  },
  {
    id: "es-clinical-followup",
    voice: VOICES.adam,
    model_id: "eleven_multilingual_v2",
    language: "es",
    context: "clinical",
    text:
      "¿Cómo se ha sentido desde la última sesión? Me dijo que la ansiedad estaba mejorando. " +
      "¿Ha podido practicar los ejercicios de respiración que repasamos?",
  },
  {
    id: "ja-greeting",
    voice: VOICES.antoni,
    model_id: "eleven_multilingual_v2",
    language: "ja",
    context: "greeting",
    text:
      "こんにちは、本日の会議へようこそ。まずは前回のアクションアイテムを確認しましょう。",
  },
];

async function loadEnv() {
  const envPath = path.join(REPO_ROOT, ".env.e2e");
  try {
    const raw = await fs.readFile(envPath, "utf8");
    for (const line of raw.split("\n")) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith("#")) continue;
      const eq = trimmed.indexOf("=");
      if (eq < 0) continue;
      const k = trimmed.slice(0, eq).trim();
      const v = trimmed.slice(eq + 1).trim();
      if (!(k in process.env)) process.env[k] = v;
    }
  } catch {
    // .env.e2e is optional — the env var can come from the shell.
  }
}

async function exists(p) {
  try {
    await fs.access(p);
    return true;
  } catch {
    return false;
  }
}

/**
 * Transcode an MP3 to 16 kHz mono signed-16 WAV — exactly the shape
 * `attune_core::transcription::local::decode_wav_to_mono_f32`
 * expects. Used so the Rust transcription integration tests can run
 * the real whisper.cpp pipeline against these fixtures without any
 * decode step of their own. Skipped if ffmpeg is missing (the WAV
 * fixtures are optional; MP3s are the source of truth).
 */
function mp3ToWav(mp3Path, wavPath) {
  return new Promise((resolve, reject) => {
    const ff = spawn(
      "ffmpeg",
      ["-y", "-i", mp3Path, "-ac", "1", "-ar", "16000", "-sample_fmt", "s16", wavPath],
      { stdio: ["ignore", "ignore", "pipe"] },
    );
    let stderr = "";
    ff.stderr.on("data", (d) => (stderr += d.toString()));
    ff.on("error", (e) => reject(e));
    ff.on("close", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`ffmpeg exited ${code}: ${stderr.slice(-300)}`));
    });
  });
}

async function hasFfmpeg() {
  return new Promise((resolve) => {
    const ff = spawn("ffmpeg", ["-version"], { stdio: "ignore" });
    ff.on("error", () => resolve(false));
    ff.on("close", (code) => resolve(code === 0));
  });
}

async function ttsViaElevenLabs({ apiKey, voice, text, model_id }) {
  const url = `https://api.elevenlabs.io/v1/text-to-speech/${voice}?output_format=mp3_44100_128`;
  const res = await fetch(url, {
    method: "POST",
    headers: {
      "xi-api-key": apiKey,
      "Content-Type": "application/json",
      Accept: "audio/mpeg",
    },
    body: JSON.stringify({
      text,
      model_id,
      voice_settings: {
        stability: 0.55,
        similarity_boost: 0.75,
        style: 0.15,
        use_speaker_boost: true,
      },
    }),
  });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`ElevenLabs ${res.status}: ${body.slice(0, 300)}`);
  }
  const buf = Buffer.from(await res.arrayBuffer());
  return buf;
}

async function main() {
  await loadEnv();
  const apiKey = process.env.ELEVENLABS_API_KEY;
  if (!apiKey) {
    console.error(
      "missing ELEVENLABS_API_KEY (put it in .env.e2e at the repo root)",
    );
    process.exit(1);
  }

  await fs.mkdir(OUTPUT_DIR, { recursive: true });

  // Load + maintain a manifest so the e2e suite can list available
  // fixtures + their metadata without re-running this script.
  let manifest = { generated_at: null, fixtures: {} };
  if (await exists(MANIFEST_PATH)) {
    try {
      manifest = JSON.parse(await fs.readFile(MANIFEST_PATH, "utf8"));
    } catch {
      // Corrupt manifest — rebuild.
    }
  }

  const ffmpegAvailable = await hasFfmpeg();
  if (!ffmpegAvailable) {
    console.warn("  (ffmpeg not found — skipping WAV transcode; MP3s only)");
  }

  let generated = 0;
  let skipped = 0;
  let wavMade = 0;

  for (const fx of CATALOG) {
    const filename = `${fx.id}.mp3`;
    const filepath = path.join(OUTPUT_DIR, filename);
    const wavName = `${fx.id}.wav`;
    const wavPath = path.join(OUTPUT_DIR, wavName);

    if (await exists(filepath)) {
      skipped += 1;
      console.log(`  skip  ${filename} (cached)`);
    } else {
      process.stdout.write(`  gen   ${filename} (${fx.language}, ${fx.text.length} chars)…`);
      const buf = await ttsViaElevenLabs({
        apiKey,
        voice: fx.voice,
        text: fx.text,
        model_id: fx.model_id,
      });
      await fs.writeFile(filepath, buf);
      generated += 1;
      process.stdout.write(` ${(buf.length / 1024).toFixed(1)} KB\n`);
    }

    // Always (re)derive the WAV when ffmpeg exists and it's missing —
    // cheap + idempotent. The WAV is the input for the Rust whisper
    // integration tests.
    let transcript_text = fx.text;
    if (ffmpegAvailable && !(await exists(wavPath))) {
      try {
        await mp3ToWav(filepath, wavPath);
        wavMade += 1;
        console.log(`  wav   ${wavName}`);
      } catch (e) {
        console.warn(`  wav   ${wavName} failed: ${e.message}`);
      }
    }

    manifest.fixtures[fx.id] = {
      file: filename,
      wav: (await exists(wavPath)) ? wavName : null,
      language: fx.language,
      context: fx.context,
      voice: fx.voice,
      chars: fx.text.length,
      // The exact prompt text is kept so the Rust transcription
      // tests can assert the whisper output contains expected
      // keywords from it (case-insensitive substring match).
      transcript_text,
    };
  }

  manifest.generated_at = new Date().toISOString();
  await fs.writeFile(MANIFEST_PATH, JSON.stringify(manifest, null, 2) + "\n");

  console.log("");
  console.log(
    `done. ${generated} mp3 generated, ${skipped} cached, ${wavMade} wav transcoded.`,
  );
  console.log(`manifest at ${path.relative(REPO_ROOT, MANIFEST_PATH)}`);
}

main().catch((e) => {
  console.error(e.stack ?? e.message ?? String(e));
  process.exit(1);
});
