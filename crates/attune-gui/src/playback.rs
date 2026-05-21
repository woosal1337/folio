//! In-app audio playback for recorded sessions.

#![allow(dead_code)]
//!
//! Wraps `rodio` so the UI can: start playing a file, pause/resume, scrub to
//! a position, and read the current position back for the progress bar. One
//! [`Player`] per app, held by `Runtime`. Switching sources stops the
//! previous sink before starting a new one.

use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use tracing::{error, warn};

/// Identifies what is currently loaded into the player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerTrack {
    pub session_dir: PathBuf,
    pub file: PathBuf,
    pub source: TrackSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackSource {
    Mic,
    System,
}

impl TrackSource {
    pub fn label(self) -> &'static str {
        match self {
            TrackSource::Mic => "Mic",
            TrackSource::System => "System",
        }
    }
}

pub struct Player {
    // The OutputStream must stay alive for as long as we want audio to play;
    // dropping it stops audio. We hold it as a member.
    _stream: OutputStream,
    handle: OutputStreamHandle,
    state: Arc<Mutex<PlayerState>>,
}

struct PlayerState {
    sink: Option<Sink>,
    track: Option<PlayerTrack>,
    duration: Option<Duration>,
    /// When we last (re)started playback at a particular file offset.
    started_at: Option<std::time::Instant>,
    /// File offset at the moment we started (or resumed). Combined with
    /// `started_at` to compute the live cursor.
    started_position: Duration,
    /// Cursor position the last time we paused. Used as `started_position`
    /// on resume.
    paused_position: Duration,
    paused: bool,
}

impl Player {
    pub fn new() -> Option<Self> {
        match OutputStream::try_default() {
            Ok((stream, handle)) => Some(Self {
                _stream: stream,
                handle,
                state: Arc::new(Mutex::new(PlayerState {
                    sink: None,
                    track: None,
                    duration: None,
                    started_at: None,
                    started_position: Duration::ZERO,
                    paused_position: Duration::ZERO,
                    paused: false,
                })),
            }),
            Err(e) => {
                warn!(error = ?e, "no default audio output device; playback disabled");
                None
            }
        }
    }

    /// Load a track and start playing from the beginning.
    pub fn play(&self, track: PlayerTrack) {
        let file = match std::fs::File::open(&track.file) {
            Ok(f) => f,
            Err(e) => {
                error!(error = %e, path = %track.file.display(), "could not open audio file");
                return;
            }
        };
        let decoder = match Decoder::new(BufReader::new(file)) {
            Ok(d) => d,
            Err(e) => {
                error!(error = %e, path = %track.file.display(), "could not decode audio file");
                return;
            }
        };
        let duration = decoder.total_duration();
        let sink = match Sink::try_new(&self.handle) {
            Ok(s) => s,
            Err(e) => {
                error!(error = ?e, "could not create audio sink");
                return;
            }
        };
        sink.append(decoder);

        let mut s = self.state.lock();
        if let Some(old) = s.sink.take() {
            old.stop();
        }
        s.sink = Some(sink);
        s.track = Some(track);
        s.duration = duration;
        s.started_at = Some(std::time::Instant::now());
        s.started_position = Duration::ZERO;
        s.paused_position = Duration::ZERO;
        s.paused = false;
    }

    pub fn toggle_pause(&self) {
        let mut s = self.state.lock();
        let Some(sink) = s.sink.as_ref() else {
            return;
        };
        if s.paused {
            sink.play();
            s.started_at = Some(std::time::Instant::now());
            s.started_position = s.paused_position;
            s.paused = false;
        } else {
            // Capture current position before pausing.
            let pos = current_position(&s);
            sink.pause();
            s.paused_position = pos;
            s.paused = true;
        }
    }

    /// Seek to a fraction `[0, 1]` of the file. Implemented by stopping the
    /// current sink and decoding a fresh source at the offset. Rodio's
    /// `try_seek` requires sources that support seeking; we go the simple
    /// "rebuild" route for portability across formats.
    pub fn seek_fraction(&self, fraction: f32) {
        let fraction = fraction.clamp(0.0, 1.0);
        let mut s = self.state.lock();
        let Some(track) = s.track.clone() else {
            return;
        };
        let Some(duration) = s.duration else {
            return;
        };
        let target = duration.mul_f32(fraction);

        let file = match std::fs::File::open(&track.file) {
            Ok(f) => f,
            Err(e) => {
                error!(error = %e, "seek: could not reopen file");
                return;
            }
        };
        let decoder = match Decoder::new(BufReader::new(file)) {
            Ok(d) => d,
            Err(e) => {
                error!(error = %e, "seek: could not decode file");
                return;
            }
        };
        let skipped = decoder.skip_duration(target);
        let sink = match Sink::try_new(&self.handle) {
            Ok(s) => s,
            Err(e) => {
                error!(error = ?e, "seek: could not create sink");
                return;
            }
        };
        sink.append(skipped);

        if let Some(old) = s.sink.take() {
            old.stop();
        }
        s.sink = Some(sink);
        s.started_at = Some(std::time::Instant::now());
        s.started_position = target;
        s.paused_position = target;
        s.paused = false;
    }

    pub fn stop(&self) {
        let mut s = self.state.lock();
        if let Some(sink) = s.sink.take() {
            sink.stop();
        }
        s.track = None;
        s.duration = None;
        s.started_at = None;
        s.started_position = Duration::ZERO;
        s.paused_position = Duration::ZERO;
        s.paused = false;
    }

    pub fn snapshot(&self) -> PlayerSnapshot {
        let s = self.state.lock();
        let position = current_position(&s);
        let duration = s.duration;
        let finished = if let Some(sink) = s.sink.as_ref() {
            sink.empty() && !s.paused
        } else {
            true
        };
        PlayerSnapshot {
            track: s.track.clone(),
            position,
            duration,
            paused: s.paused,
            finished,
        }
    }
}

fn current_position(s: &PlayerState) -> Duration {
    if s.paused {
        s.paused_position
    } else if let Some(started_at) = s.started_at {
        s.started_position + started_at.elapsed()
    } else {
        Duration::ZERO
    }
}

#[derive(Clone, Debug)]
pub struct PlayerSnapshot {
    pub track: Option<PlayerTrack>,
    pub position: Duration,
    pub duration: Option<Duration>,
    pub paused: bool,
    pub finished: bool,
}

impl PlayerSnapshot {
    pub fn is_playing(&self, file: &Path) -> bool {
        self.track
            .as_ref()
            .map(|t| t.file == file && !self.finished)
            .unwrap_or(false)
    }
    pub fn is_active_paused(&self, file: &Path) -> bool {
        self.is_playing(file) && self.paused
    }
    pub fn fraction(&self) -> f32 {
        match (self.duration, self.position) {
            (Some(d), p) if d.as_secs_f32() > 0.0 => {
                (p.as_secs_f32() / d.as_secs_f32()).clamp(0.0, 1.0)
            }
            _ => 0.0,
        }
    }
}

pub fn format_time(d: Duration) -> String {
    let total = d.as_secs();
    let m = total / 60;
    let s = total % 60;
    format!("{m:02}:{s:02}")
}
