//! Lemming central panel

use bladvak::{
    ErrorManager,
    eframe::egui::{self, CollapsingHeader, Color32, RichText},
    egui_extras::{self, syntax_highlighting::CodeTheme},
};
use gitpatch::{Line, Patch};

use crate::{app::LemmingApp, format::patch::PatchFile};

impl LemmingApp {
    /// App central panel
    pub(crate) fn app_central_panel(
        &mut self,
        ui: &mut egui::Ui,
        error_manager: &mut ErrorManager,
    ) {
        let Some(patch_file) = &self.parsed else {
            ui.label("No patch file upload");
            return;
        };

        let mut changed = false;
        ui.columns(2, |columns| {
            Self::parsed_column(&mut columns[0], patch_file);
            egui::ScrollArea::vertical()
                .id_salt("raw_column")
                .show(&mut columns[1], |ui| {
                    let mut layouter =
                        |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
                            let mut layout_job = egui_extras::syntax_highlighting::highlight(
                                ui.ctx(),
                                ui.style(),
                                &CodeTheme::dark(10.0),
                                buf.as_str(),
                                "diff",
                            );
                            layout_job.wrap.max_width = wrap_width;
                            ui.fonts_mut(|f| f.layout_job(layout_job))
                        };
                    let multiliner = egui::TextEdit::multiline(&mut self.patch_string)
                        .font(egui::FontId::monospace(12.0)) // for cursor height
                        .code_editor()
                        .desired_rows(10)
                        .lock_focus(true)
                        .desired_width(f32::INFINITY);
                    if ui.add(multiliner.layouter(&mut layouter)).changed() {
                        changed = true;
                    }
                });
        });
        if changed && let Err(e) = self.update_patch() {
            error_manager.add_error(e);
        }
    }

    /// show parsed patch
    fn parsed_column(ui: &mut egui::Ui, patch_file: &PatchFile) {
        egui::ScrollArea::both()
            .id_salt("parsed_column")
            .show(ui, |ui| {
                let mut errors = vec![];
                CollapsingHeader::new("Metadata")
                    .id_salt("metadata".to_string())
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("From");
                            ui.label(&patch_file.commit_hash);
                            ui.label("Mon Sep 17 00:00:00 2001");
                        });
                        ui.horizontal(|ui| {
                            ui.label("From: ");
                            ui.label(&patch_file.author);
                            ui.label(&patch_file.email);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Date: ");
                            ui.label(&patch_file.date);
                        });
                        ui.label(&patch_file.subject);
                        ui.label("---");
                        for one_stat in &patch_file.file_stats {
                            ui.horizontal(|ui| {
                                ui.label(&one_stat.path);
                                ui.label("|");
                                ui.label(format!("{}", one_stat.changed_lines));
                            });
                        }
                        ui.label(format!("files changes: {}", patch_file.files_changes));
                        ui.label(format!("insertions: {}", patch_file.insertions));
                        ui.label(format!("deletions: {}", patch_file.deletions));
                    });
                for (idx_diff, one_diff) in patch_file.diffs.iter().enumerate() {
                    let diff = format!(
                        "diff --git {} {}\n{}\n",
                        one_diff.old_path, one_diff.new_path, one_diff.content
                    );
                    match Patch::from_single(&diff) {
                        Ok(one_diff) => {
                            CollapsingHeader::new(format!("Diff {idx_diff}"))
                                .id_salt(format!("diff_{idx_diff}"))
                                .show(ui, |ui| {
                                    ui.label(one_diff.old.path);
                                    ui.label(one_diff.new.path);
                                    for one_hunk in one_diff.hunks {
                                        ui.separator();
                                        ui.horizontal(|ui| {
                                            ui.label("Old range:");
                                            ui.monospace(one_hunk.new_range.to_string());
                                            ui.label(" => ");
                                            ui.label("New range:");
                                            ui.monospace(one_hunk.old_range.to_string());
                                        });
                                        let mut count_modified = 0;
                                        for one_line in one_hunk.lines {
                                            let rich_text =
                                                |t: &str| RichText::new(t).monospace().size(10.0);
                                            match one_line {
                                                Line::Add(l) => {
                                                    ui.colored_label(Color32::GREEN, rich_text(l));
                                                    count_modified += 1;
                                                }
                                                Line::Context(l) => {
                                                    ui.colored_label(Color32::WHITE, rich_text(l));
                                                }
                                                Line::Remove(l) => {
                                                    ui.colored_label(Color32::RED, rich_text(l));
                                                    count_modified += 1;
                                                }
                                            }
                                        }
                                        if count_modified == 0 {
                                            errors
                                                .push(format!("No modified line for {idx_diff}"));
                                        }
                                    }
                                });
                        }
                        Err(e) => {
                            ui.label("Failed to parsed diff");
                            ui.label(e.to_string());
                        }
                    }
                }
                for one_error in errors {
                    ui.label(&one_error);
                }
            });
    }
}
