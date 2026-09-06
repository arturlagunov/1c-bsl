use zed_extension_api::{LanguageServerId, LanguageServerInstallationStatus};

pub fn checking_update(language_server_id: &LanguageServerId) {
    zed_extension_api::set_language_server_installation_status(
        language_server_id,
        &LanguageServerInstallationStatus::CheckingForUpdate,
    );
}

pub fn downloading(language_server_id: &LanguageServerId) {
    zed_extension_api::set_language_server_installation_status(
        language_server_id,
        &LanguageServerInstallationStatus::Downloading,
    );
}

pub fn failed(language_server_id: &LanguageServerId, message: &str) {
    zed_extension_api::set_language_server_installation_status(
        language_server_id,
        &LanguageServerInstallationStatus::Failed(message.to_string()),
    );
}