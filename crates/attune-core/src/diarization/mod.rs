pub mod embedding;
pub mod identify;
pub mod label;
pub mod models;
pub mod runtime;
pub mod session_speakers;

pub use embedding::{embed_speakers, embed_whole, SpeakerEmbedder};
pub use identify::{
    anchor_self_from_session, identify_session_speakers, local_device_uuid, now_ms, recording_uuid,
    SpeakerIdentification,
};
pub use label::{label_system_channel, DiarizationOutcome};
pub use models::{
    DiarizationModel, DiarizationModelStatus, DiarizationModelStore, DownloadProgress,
};
pub use runtime::{
    assign_speakers_by_overlap, DiarizationError, DiarizationOptions, DiarizationRuntime,
    DiarizedSegment,
};
pub use session_speakers::{SessionSpeaker, SessionSpeakers, SpeakerLabel, SPEAKERS_FILENAME};
