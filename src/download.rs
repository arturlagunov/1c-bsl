use std::fs;
use std::path::Path;

use zed_extension_api::{
    self as zed, DownloadedFileType, GithubReleaseOptions, LanguageServerId,
};

use crate::constants::{BSL_REPOSITORY, JAR_FILENAME};
use crate::status::Status;

/// Downloads the latest release of the BSL language server.
pub struct BslJarDownloader;

impl BslJarDownloader {
    /// Downloads the latest `bsl-language-server.jar` release to
    /// `destination`, reporting progress through the Zed installation status.
    pub fn download_to(
        language_server_id: &LanguageServerId,
        destination: &Path,
    ) -> zed::Result<()> {
        let status = Status::new(language_server_id);
        status.checking_update();

        let release = zed::latest_github_release(
            BSL_REPOSITORY,
            GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == JAR_FILENAME)
            .or_else(|| release.assets.iter().find(|asset| asset.name.ends_with("-exec.jar")))
            .ok_or_else(|| {
                format!(
                    "no `{}` or `*-exec.jar` asset found in release {} of {}",
                    JAR_FILENAME, release.version, BSL_REPOSITORY
                )
            })?;

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("failed to create {} directory: {}", parent.display(), error)
            })?;
        }

        status.downloading();

        zed::download_file(
            &asset.download_url,
            &destination.to_string_lossy(),
            DownloadedFileType::Uncompressed,
        )
        .map_err(|error| {
            status.failed(&format!("Failed to download {}: {}", JAR_FILENAME, error));
            format!("failed to download {}: {}", JAR_FILENAME, error)
        })?;

        Ok(())
    }
}
