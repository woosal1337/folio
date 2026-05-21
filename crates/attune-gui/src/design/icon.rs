//! Semantic icon set. Maps named icons to phosphor codepoints so callers
//! never see raw `char` literals. Add new icons here instead of inline.

use egui_phosphor::regular as ph;

/// Named icons used throughout the app. Each variant resolves to a single
/// glyph in the Phosphor Icons font.
#[derive(Clone, Copy, Debug)]
pub enum Icon {
    // Sidebar / navigation
    Record,
    Library,
    Transcript,
    Editor,
    CheckSquare,
    Settings,

    // Recording
    Microphone,
    SpeakerSimple,
    CircleFilled,
    Square,
    Play,
    Pause,
    Stop,

    // Actions
    Refresh,
    Folder,
    FolderOpen,
    Reveal,
    Pencil,
    Trash,
    Plus,
    Check,
    X,
    Info,
    Warning,

    // Filesystem / file types
    FileAudio,
    FileText,
    Wave,

    // Misc UI
    Cmd,
    Sparkle,
}

impl Icon {
    pub fn glyph(self) -> &'static str {
        match self {
            Icon::Record => ph::WAVEFORM,
            Icon::Library => ph::FILES,
            Icon::Transcript => ph::ARTICLE,
            Icon::Editor => ph::TEXT_ALIGN_LEFT,
            Icon::CheckSquare => ph::CHECK_SQUARE,
            Icon::Settings => ph::SLIDERS_HORIZONTAL,

            Icon::Microphone => ph::MICROPHONE,
            Icon::SpeakerSimple => ph::SPEAKER_SIMPLE_HIGH,
            Icon::CircleFilled => ph::CIRCLE,
            Icon::Square => ph::SQUARE,
            Icon::Play => ph::PLAY,
            Icon::Pause => ph::PAUSE,
            Icon::Stop => ph::STOP,

            Icon::Refresh => ph::ARROW_CLOCKWISE,
            Icon::Folder => ph::FOLDER,
            Icon::FolderOpen => ph::FOLDER_OPEN,
            Icon::Reveal => ph::ARROW_SQUARE_OUT,
            Icon::Pencil => ph::PENCIL_SIMPLE,
            Icon::Trash => ph::TRASH,
            Icon::Plus => ph::PLUS,
            Icon::Check => ph::CHECK,
            Icon::X => ph::X,
            Icon::Info => ph::INFO,
            Icon::Warning => ph::WARNING,

            Icon::FileAudio => ph::FILE_AUDIO,
            Icon::FileText => ph::FILE_TEXT,
            Icon::Wave => ph::WAVEFORM,

            Icon::Cmd => ph::COMMAND,
            Icon::Sparkle => ph::SPARKLE,
        }
    }
}
