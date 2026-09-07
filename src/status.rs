use zed_extension_api::{LanguageServerId, LanguageServerInstallationStatus};

pub struct Status<'a> {
    id: &'a LanguageServerId,
}

impl<'a> Status<'a> {
    pub fn new(id: &'a LanguageServerId) -> Self {
        Self { id }
    }

    pub fn checking_update(&self) {
        self.set(LanguageServerInstallationStatus::CheckingForUpdate);
    }

    pub fn downloading(&self) {
        self.set(LanguageServerInstallationStatus::Downloading);
    }

    pub fn failed(&self, message: &str) {
        self.set(LanguageServerInstallationStatus::Failed(message.to_string()));
    }

    fn set(&self, status: LanguageServerInstallationStatus) {
        zed_extension_api::set_language_server_installation_status(self.id, &status);
    }
}
