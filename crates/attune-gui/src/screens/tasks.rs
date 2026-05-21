//! Tasks screen: a three-column kanban (To-do / Doing / Done) sitting on
//! top of the JSON-persisted [`TaskStore`].
//!
//! Interactions:
//!   - Add a task from the top input (Enter to commit).
//!   - Click ◀ or ▶ on a card to move it left/right between columns.
//!   - Click the card body to edit title + description inline.
//!   - Trash button deletes.

use egui::{Align, Color32, Layout, RichText, Sense, Vec2};
use uuid::Uuid;

use crate::components::{caption, ghost_button_icon, mono_small};
use crate::design::tokens::{Radius, Space};
use crate::design::{palette, Icon, TextStyle};
use crate::state::{Persisted, Runtime};
use crate::tasks::{Task, TaskStatus};

pub fn show(ui: &mut egui::Ui, _ctx: &egui::Context, rt: &mut Runtime, _persisted: &mut Persisted) {
    let p = palette::current();

    // Header
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Tasks")
                .font(TextStyle::Title.font_id())
                .color(p.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!(
                    "{} open · {} done",
                    rt.tasks.count(TaskStatus::Todo) + rt.tasks.count(TaskStatus::Doing),
                    rt.tasks.count(TaskStatus::Done),
                ))
                .font(TextStyle::Caption.font_id())
                .color(p.text_muted),
            );
        });
    });
    ui.add_space(Space::sm());
    ui.label(caption(
        "Lightweight kanban for the work tied to your meetings. Stored at ~/Documents/Attune/Tasks/tasks.json.",
    ));
    ui.add_space(Space::lg());

    // Add task input row
    new_task_row(ui, rt);
    ui.add_space(Space::lg());

    // Columns
    let total_w = ui.available_width();
    let gap = Space::md();
    let col_w = ((total_w - gap * 2.0) / 3.0).max(220.0);

    ui.horizontal_top(|ui| {
        for status in TaskStatus::all() {
            ui.allocate_ui_with_layout(
                Vec2::new(col_w, ui.available_height()),
                Layout::top_down(Align::Min),
                |ui| {
                    column(ui, rt, *status);
                },
            );
            ui.add_space(gap);
        }
    });
}

fn new_task_row(ui: &mut egui::Ui, rt: &mut Runtime) {
    let p = palette::current();
    let frame = egui::Frame::default()
        .fill(p.surface_subtle)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(egui::CornerRadius::same(Radius::md() as u8))
        .inner_margin(egui::Margin::symmetric(
            Space::md() as i8,
            Space::sm() as i8,
        ));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(Icon::Plus.glyph())
                    .font(TextStyle::Body.font_id())
                    .color(p.text_muted),
            );
            ui.add_space(Space::xs());
            let mut draft = rt.tasks.draft_title.clone();
            let resp = ui.add_sized(
                [ui.available_width() - 100.0, 26.0],
                egui::TextEdit::singleline(&mut draft)
                    .frame(false)
                    .hint_text("Add a task — press Enter to save")
                    .font(TextStyle::Body.font_id()),
            );
            rt.tasks.draft_title = draft.clone();
            let pressed_enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let add_clicked = ghost_button_icon(ui, Icon::Plus, "add").clicked();
                if (pressed_enter || add_clicked) && !draft.trim().is_empty() {
                    rt.tasks.add(draft);
                    rt.tasks.draft_title.clear();
                }
            });
        });
    });
}

fn column(ui: &mut egui::Ui, rt: &mut Runtime, status: TaskStatus) {
    let p = palette::current();
    let count = rt.tasks.count(status);
    let accent = column_accent(status, &p);

    // Column header
    egui::Frame::default()
        .inner_margin(egui::Margin {
            left: Space::x2s() as i8,
            right: Space::x2s() as i8,
            top: 0,
            bottom: Space::sm() as i8,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(6.0, 14.0), Sense::hover());
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(Radius::xs() as u8),
                    accent,
                );
                ui.add_space(Space::xs());
                ui.label(
                    RichText::new(status.label())
                        .font(TextStyle::BodyStrong.font_id())
                        .color(p.text),
                );
                ui.label(
                    RichText::new(format!("{count}"))
                        .font(TextStyle::MonoSmall.font_id())
                        .color(p.text_subtle),
                );
            });
        });

    // Column body
    let frame = egui::Frame::default()
        .fill(p.surface_subtle)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(egui::CornerRadius::same(Radius::lg() as u8))
        .inner_margin(egui::Margin::same(Space::sm() as i8));
    frame.show(ui, |ui| {
        let tasks: Vec<Task> = rt.tasks.by_status(status).cloned().collect();
        if tasks.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(Space::lg());
                ui.label(
                    RichText::new("—")
                        .font(TextStyle::Caption.font_id())
                        .color(p.text_subtle),
                );
                ui.label(
                    RichText::new("nothing here")
                        .font(TextStyle::Micro.font_id())
                        .color(p.text_subtle),
                );
                ui.add_space(Space::lg());
            });
        } else {
            egui::ScrollArea::vertical()
                .id_salt(format!("col_{}", status.label()))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for task in tasks {
                        task_card(ui, rt, &task);
                        ui.add_space(Space::sm());
                    }
                });
        }
    });
}

fn column_accent(status: TaskStatus, p: &crate::design::Palette) -> Color32 {
    match status {
        TaskStatus::Todo => p.text_subtle,
        TaskStatus::Doing => p.warning,
        TaskStatus::Done => p.success,
    }
}

fn task_card(ui: &mut egui::Ui, rt: &mut Runtime, task: &Task) {
    let p = palette::current();
    let editing = rt.tasks.editing_task == Some(task.id);

    let frame = egui::Frame::default()
        .fill(p.surface_raised)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(egui::CornerRadius::same(Radius::md() as u8))
        .inner_margin(egui::Margin::symmetric(
            Space::md() as i8,
            Space::sm() as i8 + 2,
        ));
    frame.show(ui, |ui| {
        if editing {
            edit_card(ui, rt, task);
        } else {
            display_card(ui, rt, task);
        }
    });
}

fn display_card(ui: &mut egui::Ui, rt: &mut Runtime, task: &Task) {
    let p = palette::current();
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            let body = ui.label(
                RichText::new(&task.title)
                    .font(TextStyle::Body.font_id())
                    .color(p.text),
            );
            if body.clicked() {
                rt.tasks.editing_task = Some(task.id);
            }
            if !task.description.is_empty() {
                ui.label(
                    RichText::new(&task.description)
                        .font(TextStyle::Caption.font_id())
                        .color(p.text_muted),
                );
            }
            ui.label(mono_small(format!("created {}", task.created_label())));
        });
    });
    ui.add_space(Space::x2s());
    ui.horizontal(|ui| {
        if task.status != TaskStatus::Todo
            && ghost_button_icon(ui, Icon::X, task.status.prev().label()).clicked()
        {
            rt.tasks.move_to(task.id, task.status.prev());
        }
        if task.status != TaskStatus::Done
            && ghost_button_icon(ui, Icon::Check, task.status.next().label()).clicked()
        {
            rt.tasks.move_to(task.id, task.status.next());
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ghost_button_icon(ui, Icon::Trash, "").clicked() {
                rt.tasks.delete(task.id);
                if rt.tasks.editing_task == Some(task.id) {
                    rt.tasks.editing_task = None;
                }
            }
            if ghost_button_icon(ui, Icon::Pencil, "edit").clicked() {
                rt.tasks.editing_task = Some(task.id);
            }
        });
    });
}

fn edit_card(ui: &mut egui::Ui, rt: &mut Runtime, task: &Task) {
    let mut title = task.title.clone();
    let mut description = task.description.clone();
    let title_resp = ui.add(
        egui::TextEdit::singleline(&mut title)
            .font(TextStyle::BodyStrong.font_id())
            .desired_width(f32::INFINITY),
    );
    ui.add_space(Space::x2s());
    let desc_resp = ui.add(
        egui::TextEdit::multiline(&mut description)
            .font(TextStyle::Caption.font_id())
            .hint_text("description (optional)")
            .desired_rows(2)
            .desired_width(f32::INFINITY),
    );
    if title_resp.changed() {
        rt.tasks.update_title(task.id, title.clone());
    }
    if desc_resp.changed() {
        rt.tasks.update_description(task.id, description.clone());
    }
    ui.add_space(Space::x2s());
    ui.horizontal(|ui| {
        if ghost_button_icon(ui, Icon::Check, "done editing").clicked()
            || (title_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
        {
            rt.tasks.editing_task = None;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ghost_button_icon(ui, Icon::Trash, "").clicked() {
                rt.tasks.delete(task.id);
                rt.tasks.editing_task = None;
            }
        });
    });
    let _ = Uuid::nil();
}
