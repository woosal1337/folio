//! Markdown editor screen.
//!
//! Two-column layout:
//!   - Left rail: file list of `~/Documents/Attune/Notes/*.md` (or wherever
//!     the user pointed Settings → Notes folder).
//!   - Right pane: tabs for "Edit" (textarea) and "Preview" (rendered
//!     markdown via egui_commonmark), or a side-by-side split when there's
//!     enough horizontal room.

use std::path::PathBuf;

use egui::{Align, Color32, Layout, RichText, Sense, Vec2};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::components::{caption, divider, empty_state, ghost_button_icon, mono, mono_small};
use crate::design::tokens::{Radius, Space};
use crate::design::{palette, Icon, TextStyle};
use crate::notes::Note;
use crate::state::{Persisted, Runtime};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Edit,
    Split,
    Preview,
}

/// Editor-only ephemeral state. Lives on `Runtime` as a singleton so the
/// editor remembers what's open while you tab away.
pub struct EditorState {
    pub open_path: Option<PathBuf>,
    pub buffer: String,
    pub dirty: bool,
    pub view_mode: ViewMode,
    pub renaming: Option<String>,
    pub markdown_cache: CommonMarkCache,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            open_path: None,
            buffer: String::new(),
            dirty: false,
            view_mode: ViewMode::Split,
            renaming: None,
            markdown_cache: CommonMarkCache::default(),
        }
    }
}

thread_local! {
    static EDITOR_STATE: std::cell::RefCell<EditorState> = std::cell::RefCell::new(EditorState::default());
}

pub fn show(ui: &mut egui::Ui, _ctx: &egui::Context, rt: &mut Runtime, _persisted: &mut Persisted) {
    EDITOR_STATE.with(|cell| {
        let mut st = cell.borrow_mut();
        show_with_state(ui, rt, &mut st);
    });
}

fn show_with_state(ui: &mut egui::Ui, rt: &mut Runtime, st: &mut EditorState) {
    let p = palette::current();

    // Header row
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Editor")
                .font(TextStyle::Title.font_id())
                .color(p.text),
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Save indicator + actions
            if st.dirty {
                ui.label(
                    RichText::new("● unsaved")
                        .font(TextStyle::Caption.font_id())
                        .color(p.warning),
                );
                ui.add_space(Space::xs());
            } else if st.open_path.is_some() {
                ui.label(
                    RichText::new("● saved")
                        .font(TextStyle::Caption.font_id())
                        .color(p.success),
                );
                ui.add_space(Space::xs());
            }

            // View mode segmented control
            view_mode_toggle(ui, &mut st.view_mode);
        });
    });

    ui.add_space(Space::sm());
    ui.label(caption(
        "Write notes in markdown. Files live in your notes folder; edits save on every keystroke.",
    ));
    ui.add_space(Space::lg());

    let total_h = ui.available_height();
    let rail_w = 240.0;
    let gap = Space::md();

    ui.horizontal_top(|ui| {
        // File rail
        let rail_frame = egui::Frame::default()
            .fill(p.surface_subtle)
            .stroke(egui::Stroke::new(1.0, p.border))
            .corner_radius(egui::CornerRadius::same(Radius::lg() as u8))
            .inner_margin(egui::Margin::same(Space::sm() as i8));
        ui.allocate_ui_with_layout(
            Vec2::new(rail_w, total_h),
            Layout::top_down(Align::Min),
            |ui| {
                rail_frame.show(ui, |ui| {
                    ui.set_max_height(total_h);
                    file_rail(ui, rt, st);
                });
            },
        );
        ui.add_space(gap);

        // Editor pane
        let pane_w = (ui.available_width()).max(320.0);
        let pane_frame = egui::Frame::default()
            .fill(p.surface_raised)
            .stroke(egui::Stroke::new(1.0, p.border))
            .corner_radius(egui::CornerRadius::same(Radius::lg() as u8))
            .inner_margin(egui::Margin::symmetric(
                Space::lg() as i8,
                Space::md() as i8,
            ));
        ui.allocate_ui_with_layout(
            Vec2::new(pane_w, total_h),
            Layout::top_down(Align::Min),
            |ui| {
                pane_frame.show(ui, |ui| {
                    editor_pane(ui, rt, st);
                });
            },
        );
    });
}

fn view_mode_toggle(ui: &mut egui::Ui, mode: &mut ViewMode) {
    let p = palette::current();
    let frame = egui::Frame::default()
        .fill(p.surface_subtle)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(egui::CornerRadius::same(Radius::md() as u8))
        .inner_margin(egui::Margin::same(2));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            for (m, label) in [
                (ViewMode::Edit, "Edit"),
                (ViewMode::Split, "Split"),
                (ViewMode::Preview, "Preview"),
            ] {
                let selected = *mode == m;
                let bg = if selected {
                    p.surface_overlay
                } else {
                    Color32::TRANSPARENT
                };
                let text_color = if selected { p.text } else { p.text_muted };
                let resp = ui.add(
                    egui::Button::new(
                        RichText::new(label)
                            .font(TextStyle::Caption.font_id())
                            .color(text_color),
                    )
                    .corner_radius(egui::CornerRadius::same(Radius::sm() as u8))
                    .fill(bg)
                    .stroke(egui::Stroke::NONE)
                    .min_size(Vec2::new(54.0, 22.0)),
                );
                if resp.clicked() {
                    *mode = m;
                }
            }
        });
    });
}

fn file_rail(ui: &mut egui::Ui, rt: &mut Runtime, st: &mut EditorState) {
    let p = palette::current();

    // Header: title + new + refresh
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("NOTES")
                .font(TextStyle::Caption.font_id())
                .color(p.text_muted),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ghost_button_icon(ui, Icon::Refresh, "").clicked() {
                rt.notes.refresh();
            }
            if ghost_button_icon(ui, Icon::Plus, "new").clicked() {
                if let Ok(note) = rt.notes.create_new() {
                    open_note(rt, st, &note);
                    rt.notes.refresh();
                }
            }
        });
    });
    ui.add_space(Space::sm());

    let notes = rt.notes.notes.clone();
    if notes.is_empty() {
        ui.add_space(Space::md());
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(Icon::FileText.glyph())
                    .font(egui::FontId::new(22.0, egui::FontFamily::Proportional))
                    .color(p.text_subtle),
            );
            ui.add_space(Space::xs());
            ui.label(
                RichText::new("No notes yet")
                    .font(TextStyle::Caption.font_id())
                    .color(p.text_muted),
            );
            ui.label(
                RichText::new("Click + to create one")
                    .font(TextStyle::Micro.font_id())
                    .color(p.text_subtle),
            );
        });
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for note in &notes {
                file_row(ui, note, st, rt);
            }
        });
}

fn file_row(ui: &mut egui::Ui, note: &Note, st: &mut EditorState, rt: &mut Runtime) {
    let p = palette::current();
    let selected = st.open_path.as_deref() == Some(&note.path);

    let height = 44.0;
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());

    let bg = if selected {
        p.surface_overlay
    } else if resp.hovered() {
        p.surface
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter();
    painter.rect_filled(rect, egui::CornerRadius::same(Radius::sm() as u8), bg);

    let title_color = if selected { p.text } else { p.text_muted };
    let title_font = if selected {
        TextStyle::BodyStrong.font_id()
    } else {
        TextStyle::Body.font_id()
    };

    let title_galley = painter.layout_no_wrap(note.title.clone(), title_font, title_color);
    let title_pos = egui::pos2(rect.min.x + Space::sm() + 2.0, rect.min.y + 6.0);
    painter.galley(title_pos, title_galley, title_color);

    let meta = format!("{}  ·  {} B", note.modified_label(), note.size_bytes);
    let meta_galley = painter.layout_no_wrap(meta, TextStyle::Micro.font_id(), p.text_subtle);
    let meta_pos = egui::pos2(rect.min.x + Space::sm() + 2.0, rect.min.y + 25.0);
    painter.galley(meta_pos, meta_galley, p.text_subtle);

    if selected {
        let bar_x = rect.min.x + 1.0;
        painter.line_segment(
            [
                egui::pos2(bar_x, rect.center().y - 10.0),
                egui::pos2(bar_x, rect.center().y + 10.0),
            ],
            egui::Stroke::new(2.0, p.text),
        );
    }

    if resp.clicked() {
        open_note(rt, st, note);
    }

    ui.add_space(2.0);
}

fn open_note(rt: &mut Runtime, st: &mut EditorState, note: &Note) {
    // Save any pending changes to the previously open note first.
    if st.dirty {
        if let Some(old) = st.open_path.clone() {
            let _ = rt.notes.write(&old, &st.buffer);
        }
    }
    match rt.notes.read(&note.path) {
        Ok(body) => {
            st.buffer = body;
            st.open_path = Some(note.path.clone());
            st.dirty = false;
            st.renaming = None;
        }
        Err(e) => {
            rt.last_error = Some(format!("could not open {}: {e}", note.path.display()));
        }
    }
}

fn editor_pane(ui: &mut egui::Ui, rt: &mut Runtime, st: &mut EditorState) {
    let p = palette::current();
    let path = match st.open_path.clone() {
        Some(p) => p,
        None => {
            empty_state(
                ui,
                Icon::Editor,
                "Select or create a note",
                "Open one from the list on the left, or click + to start fresh.",
            );
            return;
        }
    };

    // File header with rename + delete
    ui.horizontal(|ui| {
        let current_title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled")
            .to_string();

        let renaming_active = st.renaming.is_some();
        if renaming_active {
            let mut working = st.renaming.clone().unwrap_or_default();
            let resp = ui.add(
                egui::TextEdit::singleline(&mut working)
                    .font(TextStyle::Heading.font_id())
                    .desired_width(300.0),
            );
            st.renaming = Some(working.clone());

            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Ok(new_path) = rt.notes.rename(&path, &working) {
                    st.open_path = Some(new_path);
                }
                st.renaming = None;
                rt.notes.refresh();
            } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                st.renaming = None;
            }
        } else {
            ui.label(
                RichText::new(current_title.clone())
                    .font(TextStyle::Heading.font_id())
                    .color(p.text),
            );
            if ghost_button_icon(ui, Icon::Pencil, "rename").clicked() {
                st.renaming = Some(current_title);
            }
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ghost_button_icon(ui, Icon::Trash, "delete").clicked() {
                if rt.notes.delete(&path).is_ok() {
                    st.open_path = None;
                    st.buffer.clear();
                    st.dirty = false;
                }
                rt.notes.refresh();
            }
        });
    });

    ui.add_space(Space::xs());
    ui.label(mono_small(path.display().to_string()).color(p.text_subtle));
    ui.add_space(Space::sm());
    divider(ui);
    ui.add_space(Space::sm());

    let available_h = ui.available_height();
    let available_w = ui.available_width();
    let split = matches!(st.view_mode, ViewMode::Split);
    let show_edit = matches!(st.view_mode, ViewMode::Edit | ViewMode::Split);
    let show_preview = matches!(st.view_mode, ViewMode::Preview | ViewMode::Split);

    if split {
        ui.horizontal(|ui| {
            let half = available_w * 0.5 - Space::sm();
            ui.allocate_ui_with_layout(
                Vec2::new(half, available_h),
                Layout::top_down(Align::Min),
                |ui| {
                    if show_edit {
                        editor_textarea(ui, rt, st, available_h, &path);
                    }
                },
            );
            divider_vertical(ui, available_h);
            ui.allocate_ui_with_layout(
                Vec2::new(half, available_h),
                Layout::top_down(Align::Min),
                |ui| {
                    if show_preview {
                        preview_pane(ui, st);
                    }
                },
            );
        });
    } else if show_edit {
        editor_textarea(ui, rt, st, available_h, &path);
    } else {
        preview_pane(ui, st);
    }
}

fn divider_vertical(ui: &mut egui::Ui, height: f32) {
    let p = palette::current();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, height), Sense::hover());
    ui.painter().line_segment(
        [rect.min, egui::pos2(rect.min.x, rect.min.y + height)],
        egui::Stroke::new(1.0, p.border),
    );
}

fn editor_textarea(
    ui: &mut egui::Ui,
    rt: &mut Runtime,
    st: &mut EditorState,
    height: f32,
    path: &std::path::Path,
) {
    let resp = egui::ScrollArea::vertical()
        .id_salt("editor_area")
        .auto_shrink([false, false])
        .max_height(height)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut st.buffer)
                    .desired_width(f32::INFINITY)
                    .desired_rows(20)
                    .frame(false)
                    .font(TextStyle::Mono.font_id())
                    .lock_focus(true),
            )
        });

    if resp.inner.changed() {
        st.dirty = true;
        // Autosave on each change. Throttle later if it becomes a problem.
        if let Err(e) = rt.notes.write(path, &st.buffer) {
            rt.last_error = Some(format!("autosave failed: {e}"));
        } else {
            st.dirty = false;
        }
    }
}

fn preview_pane(ui: &mut egui::Ui, st: &mut EditorState) {
    egui::ScrollArea::vertical()
        .id_salt("preview_area")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            CommonMarkViewer::new().show(ui, &mut st.markdown_cache, &st.buffer);
        });
}

// Mono not used here directly; keep the import warm so the file compiles
// cleanly without dropping the helper for future tweaks.
#[allow(dead_code)]
fn _mono_keepalive(text: &str) -> egui::RichText {
    mono(text)
}
