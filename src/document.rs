//! Lemming document

use std::path::{Path, PathBuf};

use bladvak::{AppError, utils::document::DocumentTrait};

use crate::patch::{PatchFile, parse_file};

/// Single document
#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
pub(crate) struct Document {
    /// Current patch
    pub(crate) patch_string: String,

    /// Current patch filename
    pub(crate) filename: PathBuf,

    /// Parsed patch
    #[serde(skip)]
    pub(crate) parsed: Option<PatchFile>,
}

impl DocumentTrait for Document {
    fn path(&self) -> &Path {
        &self.filename
    }
}

impl Document {
    /// pare the patch
    pub(crate) fn update_patch(&mut self) -> Result<(), AppError> {
        self.parsed = None;
        let (_, patch_file) = parse_file(&self.patch_string)
            .map_err(|e| format!("Error during patch parsing {e}"))?;
        self.parsed = Some(patch_file);
        Ok(())
    }
}
