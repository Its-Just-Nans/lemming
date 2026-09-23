//! Lemming app

use std::path::PathBuf;

use bladvak::{
    AppError, BladvakApp, ErrorManager, File,
    eframe::{CreationContext, egui},
    utils::{Documents, is_native},
};

use crate::{document::Document, panels::Info};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(default)]
pub struct LemmingApp {
    /// List of documents
    pub(crate) documents: Documents<Document>,
}

/// Demo patch
const DEMO_PATCH: &str = include_str!("../tests/a.patch");

impl Default for LemmingApp {
    fn default() -> Self {
        let (filename, patch_string) = Self::load_default();
        let document = Document {
            patch_string,
            filename,
            parsed: None,
        };
        let mut documents = Documents::default();
        documents.push(document);
        Self { documents }
    }
}

impl LemmingApp {
    /// Load the default demo patch
    pub(crate) fn load_default() -> (PathBuf, String) {
        (PathBuf::from("demo.patch"), DEMO_PATCH.to_string())
    }
}

impl BladvakApp<'_> for LemmingApp {
    fn side_panel(&mut self, ui: &mut egui::Ui, func_ui: impl FnOnce(&mut egui::Ui, &mut Self)) {
        egui::Frame::central_panel(&ui.ctx().global_style())
            .show(ui, |panel_ui| func_ui(panel_ui, self));
    }

    fn panel_list(&self) -> Vec<Box<dyn bladvak::app::BladvakPanel<App = Self>>> {
        vec![Box::new(Info)]
    }

    fn is_side_panel(&self) -> bool {
        true
    }

    fn is_open_button(&self) -> bool {
        true
    }

    fn handle_file(&mut self, file: File) -> Result<(), AppError> {
        let mut document = Document {
            patch_string: String::from_utf8_lossy(&file.data).to_string(),
            filename: file.path,
            parsed: None,
        };
        if let Err(_e) = document.update_patch() {
            document.patch_string.clear();
            return Err(format!(
                "Parsing error while parsing the file {}",
                document.filename.display()
            )
            .into());
        }
        self.documents.push(document);
        Ok(())
    }

    fn top_panel(&mut self, ui: &mut egui::Ui, _error_manager: &mut ErrorManager) {
        self.documents.show_file_list(ui);
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
        &include_bytes!("../assets/icon-256.png")[..]
    }

    fn try_new_with_args(
        mut saved_state: Self,
        _cc: &CreationContext<'_>,
        args: &[String],
        error_manager: &mut ErrorManager,
    ) -> Result<Self, AppError> {
        if is_native() && args.len() > 1 {
            use std::fs;
            let mut app = saved_state;
            // do not clear the documents since we save them
            // app.documents.clear();
            for one_path in &args[1..] {
                let absolute_path = fs::canonicalize(one_path)
                    .map_err(|e| format!("Unable to canonicalize path '{one_path}': {e}"))?;
                let bytes = std::fs::read(&absolute_path).map_err(|e| {
                    format!("Unable to read file '{}': {e}", absolute_path.display())
                })?;
                let document = Document {
                    patch_string: String::from_utf8_lossy(&bytes).to_string(),
                    filename: absolute_path,
                    parsed: None,
                };
                app.documents.push(document);
            }
            // update all
            for one_doc in &mut app.documents {
                // Try to parse the patch file
                if let Err(e) = one_doc.update_patch() {
                    error_manager.add_error(e);
                }
            }
            Ok(app)
        } else {
            if saved_state.documents.is_some() {
                for one_doc in &mut saved_state.documents {
                    // Try to parse the patch file
                    if let Err(e) = one_doc.update_patch() {
                        error_manager.add_error(e);
                    }
                }
            }
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
            .map_err(|e| std::io::Error::other(format!("Error check_on_patch_file() {e}")))?;
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
