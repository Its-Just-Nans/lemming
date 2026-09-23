//! panels

use bladvak::{BladvakApp, app::BladvakPanel, eframe::egui};

use crate::LemmingApp;

/// Panel for settings
#[derive(Debug)]
pub(crate) struct Info;

impl BladvakPanel for Info {
    type App = LemmingApp;

    fn name(&self) -> &'static str {
        "Infos"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn has_ui(&self) -> bool {
        false
    }

    fn ui_settings(
        &self,
        app: &mut Self::App,
        ui: &mut egui::Ui,
        error_manager: &mut bladvak::ErrorManager,
    ) {
        if ui.button("Default patch").clicked() {
            let (filename, patch_string) = LemmingApp::load_default();
            if let Err(err) = app.handle_file(bladvak::File {
                data: patch_string.into_bytes(),
                path: filename,
            }) {
                error_manager.add_error(err);
            }
        }
    }

    fn ui(
        &self,
        _app: &mut Self::App,
        _ui: &mut egui::Ui,
        _error_manager: &mut bladvak::ErrorManager,
    ) {
    }
}
