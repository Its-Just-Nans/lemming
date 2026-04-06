//! Lemming app

use bladvak::{
    AppError, BladvakApp, ErrorManager, File,
    eframe::{CreationContext, egui},
    utils::is_native,
};
use std::path::PathBuf;

use crate::format::patch::{PatchFile, parse_patch};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
#[serde(default)]
pub struct LemmingApp {
    /// Current patch
    pub patch_string: String,
    /// Current patch filename
    pub filename: PathBuf,

    /// Parsed patch
    #[serde(skip)]
    pub parsed: Option<PatchFile>,
}

impl LemmingApp {
    /// parse patch
    pub(crate) fn update_patch(&mut self) -> Result<(), AppError> {
        let (_, patch_file) = parse_patch(&self.patch_string)
            .map_err(|e| format!("Error during patch parsing {e}"))?;
        self.parsed = Some(patch_file);
        Ok(())
    }
}

impl BladvakApp<'_> for LemmingApp {
    fn side_panel(&mut self, ui: &mut egui::Ui, func_ui: impl FnOnce(&mut egui::Ui, &mut Self)) {
        egui::Frame::central_panel(&ui.ctx().global_style()).show(ui, |panel_ui| func_ui(panel_ui, self));
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
        self.update_patch()?;
        Ok(())
    }

    fn top_panel(&mut self, ui: &mut egui::Ui, _error_manager: &mut ErrorManager) {
        ui.label("Filename:");
        ui.label(format!("{}", self.filename.display()));
    }

    fn menu_file(&mut self, ui: &mut egui::Ui, _error_manager: &mut ErrorManager) {
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
        saved_state: Self,
        _cc: &CreationContext<'_>,
        args: &[String],
    ) -> Result<Self, AppError> {
        if is_native() && args.len() > 1 {
            let path = &args[1];
            let bytes = std::fs::read(path)?;
            let mut app = saved_state;
            app.handle_file(File {
                data: bytes,
                path: PathBuf::from(path),
            })?;
            Ok(app)
        } else {
            Ok(saved_state)
        }
    }
}


#[cfg(test)]
mod tests {

    #[test]
    fn test_panic_file() {
        use gitpatch::Patch;

        let sample = include_str!("../panic.patch");
        let patch = Patch::from_single(sample);
        assert!(patch.is_err());
    }
}
