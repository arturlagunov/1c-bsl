use zed_extension_api::{self as zed, Command, Result, Worktree};

struct BslExtension;

impl zed::Extension for BslExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        _worktree: &Worktree,
    ) -> Result<Command> {
        Ok(Command {
            command: "java".into(),
            args: vec![
                "-Xmx4g".into(),
                "-jar".into(),
                "/home/artur/.local/lib/bsl-language-server/bsl-language-server.jar".into(),
            ],
            env: Vec::new(),
        })
    }
}

zed::register_extension!(BslExtension);