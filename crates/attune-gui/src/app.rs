//! App shell: sidebar + content area. Dispatches to screens.

use egui::{Align, Layout, RichText};

use crate::components::{divider, micro, nav_item};
use crate::design::tokens::{Layout as L, Space};
use crate::design::{palette, Icon, TextStyle};
use crate::screens;
use crate::state::{self, Persisted, Runtime, Screen};

pub struct App {
    persisted: Persisted,
    rt: Runtime,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut persisted: Persisted = cc
            .storage
            .and_then(|s| eframe::get_value::<Persisted>(s, eframe::APP_KEY))
            .unwrap_or_default();
        persisted.active_screen = persisted.active_screen.migrated();

        // Apply the persisted theme so the very first paint matches.
        crate::design::set_theme_and_apply(&cc.egui_ctx, persisted.theme);

        let mut rt = Runtime::new(&persisted);
        state::refresh_devices(&mut rt, &mut persisted);
        state::refresh_history(&mut rt, &persisted.output_dir);

        Self { persisted, rt }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.persisted);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // While recording, keep ticking so the timer + pulse animate.
        if self.rt.is_recording() {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }

        let p = palette::current();

        // Sidebar
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(L::sidebar_width())
            .frame(egui::Frame::default().fill(p.surface_subtle).inner_margin(
                egui::Margin::symmetric(Space::md() as i8, Space::lg() as i8),
            ))
            .show(ctx, |ui| {
                self.draw_sidebar(ui);
            });

        // Content area
        let full_width = self.persisted.active_screen.wants_full_width();
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(p.surface)
                    .inner_margin(egui::Margin::symmetric(
                        Space::x2l() as i8,
                        Space::xl() as i8,
                    )),
            )
            .show(ctx, |ui| {
                if full_width {
                    self.draw_content(ui, ctx);
                } else {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let max_w = L::content_max_width();
                            let avail = ui.available_width();
                            let pad = ((avail - max_w) * 0.5).max(0.0);
                            ui.horizontal(|ui| {
                                ui.add_space(pad);
                                ui.vertical(|ui| {
                                    ui.set_max_width(max_w.min(avail));
                                    self.draw_content(ui, ctx);
                                });
                            });
                        });
                }
            });
    }
}

impl App {
    fn draw_sidebar(&mut self, ui: &mut egui::Ui) {
        let p = palette::current();

        // Brand
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("attune")
                    .font(TextStyle::Title.font_id())
                    .color(p.text)
                    .strong(),
            );
        });
        ui.label(micro("local meeting capture").color(p.text_subtle));
        ui.add_space(Space::xl());

        // Recording status (compact)
        if self.rt.is_recording() {
            let t = ui.ctx().input(|i| i.time);
            let pulse = 0.55 + 0.45 * (t * 3.2).sin();
            let alpha = (pulse.clamp(0.0, 1.0) * 255.0) as u8;
            let glow = egui::Color32::from_rgba_unmultiplied(
                p.danger.r(),
                p.danger.g(),
                p.danger.b(),
                alpha,
            );
            ui.horizontal(|ui| {
                let dot_size = 8.0;
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(dot_size), egui::Sense::hover());
                ui.painter()
                    .circle_filled(rect.center(), dot_size * 0.85, glow);
                ui.painter()
                    .circle_filled(rect.center(), dot_size * 0.42, p.danger);
                ui.add_space(Space::xs());
                ui.label(
                    RichText::new("recording")
                        .font(TextStyle::Caption.font_id())
                        .color(p.text),
                );
                ui.add_space(Space::xs());
                ui.label(
                    RichText::new(self.rt.elapsed_label())
                        .font(TextStyle::MonoSmall.font_id())
                        .color(p.text),
                );
            });
            ui.add_space(Space::md());
        }

        // Nav items
        for screen in Screen::all() {
            let (icon, label) = match screen {
                Screen::Record => (Icon::Record, "Record"),
                Screen::Library => (Icon::Library, "Library"),
                Screen::Editor => (Icon::Editor, "Editor"),
                Screen::Tasks => (Icon::CheckSquare, "Tasks"),
                Screen::Settings => (Icon::Settings, "Settings"),
                Screen::Transcripts => continue,
            };
            let is_active = self.persisted.active_screen == *screen;
            if nav_item(ui, icon, label, is_active).clicked() {
                self.persisted.active_screen = *screen;
            }
            ui.add_space(2.0);
        }

        // Footer pinned to bottom
        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            ui.add_space(Space::xs());
            ui.label(
                RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .font(TextStyle::Micro.font_id())
                    .color(p.text_subtle),
            );
            ui.label(
                RichText::new("audio stays on this mac")
                    .font(TextStyle::Micro.font_id())
                    .color(p.text_subtle),
            );
            ui.add_space(Space::sm());
            divider(ui);
        });
    }

    fn draw_content(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        match self.persisted.active_screen {
            Screen::Record => screens::record::show(ui, ctx, &mut self.rt, &mut self.persisted),
            // Transcripts is merged into Library; any stale routing lands here too.
            Screen::Library | Screen::Transcripts => {
                screens::library::show(ui, ctx, &mut self.rt, &mut self.persisted)
            }
            Screen::Editor => screens::editor::show(ui, ctx, &mut self.rt, &mut self.persisted),
            Screen::Tasks => screens::tasks::show(ui, ctx, &mut self.rt, &mut self.persisted),
            Screen::Settings => screens::settings::show(ui, ctx, &mut self.rt, &mut self.persisted),
        }
    }
}
