//! Lemming central panel

use bladvak::{
    ErrorManager,
    eframe::egui::{self, CollapsingHeader, Color32, RichText},
    egui_extras::{self, syntax_highlighting::CodeTheme},
};
use gitpatch::{Line, Patch};

use crate::app::LemmingApp;

impl LemmingApp {
    /// App central panel
    pub(crate) fn app_central_panel(
        &mut self,
        ui: &mut egui::Ui,
        _error_manager: &mut ErrorManager,
    ) {
        let mut changed = false;
        let mut patch_errors = if let Some(error) = &self.parsing_error {
            vec![(Color32::RED, error.clone())]
        } else {
            vec![]
        };
        ui.columns(2, |columns| {
            patch_errors.extend(self.parsed_column(&mut columns[0]));
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
        if changed {
            if let Err(e) = self.update_patch() {
                self.parsing_error = Some(e.to_string());
            } else {
                self.parsing_error = None;
            }
        }
        if !patch_errors.is_empty() {
            egui::Window::new("Patch errors")
                .vscroll(true)
                .show(ui.ctx(), |ui| {
                    for (label_color, one_error) in patch_errors {
                        ui.colored_label(label_color, &one_error);
                    }
                });
        }
    }

    /// show parsed patch
    #[allow(clippy::too_many_lines)] // maybe reformat later
    fn parsed_column(&mut self, ui: &mut egui::Ui) -> Vec<(Color32, String)> {
        let Some(patch_file) = &self.parsed else {
            if self.parsing_error.is_some() {
                ui.label("Error during patch parsing");
            } else {
                ui.label("No patch file upload");
            }
            return vec![];
        };
        let mut errors = vec![];
        egui::ScrollArea::both()
            .id_salt("parsed_column")
            .show(ui, |ui| {
                if let Some(metadata) = &patch_file.metadata {
                CollapsingHeader::new("Metadata")
                    .id_salt("metadata".to_string())
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("From");
                            ui.label(&metadata.commit_hash);
                            ui.label("Mon Sep 17 00:00:00 2001");
                        });
                        ui.horizontal(|ui| {
                            ui.label("From: ");
                            ui.label(&metadata.author);
                            ui.label(&metadata.email);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Date: ");
                            ui.label(&metadata.date);
                        });
                        ui.label(&metadata.subject);
                        ui.label("---");
                        for one_stat in &metadata.file_stats {
                            ui.horizontal(|ui| {
                                ui.label(&one_stat.path);
                                ui.label("|");
                                ui.label(format!("{}", one_stat.changed_lines));
                            });
                        }
                        ui.label(format!("files changes: {}", metadata.files_changes));
                        ui.label(format!("insertions: {}", metadata.insertions));
                        ui.label(format!("deletions: {}", metadata.deletions));
                        for one_line in &metadata.more_file_stats {
                                ui.label(one_line);
                        }
                    });
                }
                for (idx_diff, one_diff) in patch_file.diffs.iter().enumerate() {
                    let diff = format!(
                        "diff --git {} {}\n{}\n",
                        one_diff.old_path, one_diff.new_path, one_diff.content
                    );
                    let is_deletion = one_diff.content.starts_with("deleted");
                    match Patch::from_single(&diff) {
                        Ok(one_diff) => {
                            CollapsingHeader::new(format!("Diff {idx_diff}"))
                                .id_salt(format!("diff_{idx_diff}"))
                                .show(ui, |ui| {
                                    ui.label(one_diff.old.path);
                                    ui.label(one_diff.new.path);
                                    for (idx_hunk, one_hunk) in one_diff.hunks.into_iter().enumerate() {
                                        ui.separator();
                                        ui.horizontal(|ui| {
                                            ui.label("Old range:");
                                            ui.monospace(one_hunk.old_range.to_string());
                                            ui.label(" => ");
                                            ui.label("New range:");
                                            ui.monospace(one_hunk.new_range.to_string());
                                        });
                                        let mut count_modified = 0;
                                        let mut check_new_range_count = one_hunk.old_range.count;
                                        let mut check_old_range_count = 0;
                                        if one_hunk.lines.len() >= 3 {
                                            let first_three = &one_hunk.lines[..3];

                                            let first_ok = first_three.iter().all(|l| matches!(l, Line::Context(_)));

                                            if !is_deletion && !first_ok {
                                                errors.push((Color32::ORANGE, format!("Diff n{idx_diff} hunk n{idx_hunk}: Missing the three first context lines")));
                                            }
                                        }
                                            let rich_text = |t: &str| RichText::new(t).monospace().size(10.0);
                                        for one_line in one_hunk.lines {
                                            match one_line {
                                                Line::Add(l) => {
                                                    ui.colored_label(Color32::GREEN, rich_text(l));
                                                    count_modified += 1;
                                                    check_new_range_count += 1;
                                                }
                                                Line::Context(l) => {
                                                    ui.colored_label(Color32::WHITE, rich_text(l));
                                                    check_old_range_count += 1;
                                                }
                                                Line::Remove(l) => {
                                                    ui.colored_label(Color32::RED, rich_text(l));
                                                    count_modified += 1;
                                                    check_new_range_count -= 1;
                                                    check_old_range_count += 1;
                                                }
                                            }
                                        }
                                        if count_modified == 0 {
                                            errors.push((Color32::RED, format!("Diff n{idx_diff} hunk n{idx_hunk}: No modified line")));
                                        }
                                        if check_new_range_count != one_hunk.new_range.count {
                                            errors.push((Color32::RED, format!("Diff n{idx_diff} hunk n{idx_hunk}: Invalid new range")));
                                        }
                                        if check_old_range_count != one_hunk.old_range.count {
                                            errors.push((Color32::RED, format!("Diff n{idx_diff} hunk n{idx_hunk}: Invalid old range")));
                                        }
                                    }
                                });
                        }
                        Err(e) => {
                            let msg =format!("Failed to parse diff n{idx_diff}");
                            ui.label(&msg);
                            errors.push((Color32::RED, msg));
                            ui.label(e.to_string());
                        }
                    }
                }
            });
        errors
    }
}
