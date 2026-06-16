//! Lemming app

use bladvak::{
    AppError, BladvakApp, ErrorManager, File,
    eframe::{CreationContext, egui},
    utils::is_native,
};
use std::path::PathBuf;

use crate::patch::{PatchFile, parse_file};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
#[serde(default)]
pub struct LemmingApp {
    /// Current patch
    pub(crate) patch_string: String,

    /// Current patch filename
    pub(crate) filename: PathBuf,

    /// Parsed patch
    #[serde(skip)]
    pub(crate) parsed: Option<PatchFile>,

    /// Parsing error
    #[serde(skip)]
    pub(crate) parsing_error: Option<String>,
}

impl LemmingApp {
    /// parse patch
    pub(crate) fn update_patch(&mut self) -> Result<(), AppError> {
        self.parsed = None;
        let (_, patch_file) = parse_file(&self.patch_string)
            .map_err(|e| format!("Error during patch parsing {e}"))?;
        self.parsed = Some(patch_file);
        Ok(())
    }

    /// Handle the saved state
    pub(crate) fn handle_saved_state(&mut self) {
        if !self.patch_string.is_empty()
            && let Err(e) = self.update_patch()
        {
            self.parsing_error = Some(e.to_string());
        }
    }
}

impl BladvakApp<'_> for LemmingApp {
    fn side_panel(&mut self, ui: &mut egui::Ui, func_ui: impl FnOnce(&mut egui::Ui, &mut Self)) {
        egui::Frame::central_panel(&ui.ctx().global_style())
            .show(ui, |panel_ui| func_ui(panel_ui, self));
    }

    fn panel_list(&self) -> Vec<Box<dyn bladvak::app::BladvakPanel<App = Self>>> {
        vec![]
    }

    fn is_side_panel(&self) -> bool {
        true
    }

    fn is_open_button(&self) -> bool {
        true
    }

    fn handle_file(&mut self, file: File) -> Result<(), AppError> {
        self.patch_string = String::from_utf8_lossy(&file.data).to_string();
        self.filename = file.path;
        if let Err(e) = self.update_patch() {
            self.parsing_error = Some(e.to_string());
        }
        Ok(())
    }

    fn top_panel(&mut self, ui: &mut egui::Ui, _error_manager: &mut ErrorManager) {
        ui.label("Filename:");
        ui.label(format!("{}", self.filename.display()));
    }

    fn menu_file(&mut self, _ui: &mut egui::Ui, _error_manager: &mut ErrorManager) {
        // self.app_menu_file(ui, error_manager);
    }

    fn central_panel(&mut self, ui: &mut egui::Ui, error_manager: &mut ErrorManager) {
        self.app_central_panel(ui, error_manager);
    }

    fn name() -> String {
        env!("CARGO_PKG_NAME").to_string()
    }

    fn version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn repo_url() -> String {
        "https://github.com/Its-Just-Nans/lemming".to_string()
    }

    fn icon() -> &'static [u8] {
        // &include_bytes!("../assets/icon-256.png")[..]
        &[]
    }

    fn try_new_with_args(
        mut saved_state: Self,
        _cc: &CreationContext<'_>,
        args: &[String],
    ) -> Result<Self, AppError> {
        if is_native() && args.len() > 1 {
            use std::fs;
            let path = &args[1];
            let absolute_path = fs::canonicalize(path)?;
            let bytes = fs::read(&absolute_path)?;
            let mut app = saved_state;
            app.handle_file(File {
                data: bytes,
                path: absolute_path,
            })?;
            Ok(app)
        } else {
            saved_state.handle_saved_state();
            Ok(saved_state)
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_panic_file() {
        use gitpatch::Patch;

        let sample = include_str!("../tests/panic.diff");
        let patch = Patch::from_single(sample);
        assert!(patch.is_err());
    }

    fn check_on_patch_file(patch_content: &str) -> std::io::Result<()> {
        use crate::central_panel::check_patch;
        use crate::patch::parse_file;
        use gitpatch::Patch;

        let (_, patch_file) = parse_file(patch_content)
            .map_err(|e| std::io::Error::other(format!("Error while patch parsing {e}")))?;
        for (idx_diff, one_diff) in patch_file.diffs.iter().enumerate() {
            let content = if one_diff.content.ends_with('\n') {
                one_diff
                    .content
                    .strip_suffix("\n")
                    .unwrap_or(&one_diff.content)
            } else {
                &one_diff.content
            };
            let diff = format!(
                "diff --git {} {}\n{}\n",
                one_diff.old_path, one_diff.new_path, content
            );
            let is_deletion = content.starts_with("deleted");
            match Patch::from_single(&diff) {
                Ok(one_diff) => {
                    if let Some(_diff_errors) = check_patch(idx_diff, &one_diff, is_deletion) {
                        return Err(std::io::Error::other("Diff inside patch contains error"));
                    }
                }
                Err(_err) => {
                    return Err(std::io::Error::other("Patch contains error"));
                }
            }
        }
        Ok(())
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_read_patch() {
        let file_content = include_str!("../tests/a.patch");
        check_on_patch_file(file_content).unwrap();

        let file_content = include_str!("../tests/b.patch");
        check_on_patch_file(file_content).unwrap();

        let file_content = include_str!("../tests/cd2e2edd49aef7dccfcf1c5f2bff50fa4d4627a9.patch");
        check_on_patch_file(file_content).unwrap();
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_read_patch_fail() {
        let file_content = include_str!("../tests/b_icnal.patch");
        check_on_patch_file(file_content).unwrap_err();
    }
}
