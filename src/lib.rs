use std::{
    env, fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use miniz_oxide::inflate;
use zed_extension_api::{
    self as zed,
    process::Command as ProcessCommand,
    settings::LspSettings,
    Command, GithubReleaseOptions, LanguageServerId, LanguageServerInstallationStatus, Result,
    Worktree,
};

const LANGUAGE_SERVER_ID: &str = "bsl";
const LANGUAGE_SERVER_NAME: &str = "bsl-language-server";
const BSL_REPOSITORY: &str = "1c-syntax/bsl-language-server";
const BSL_JAR_FILENAME: &str = "bsl-language-server.jar";
const MANAGED_JAR_DIRECTORY: &str = "binaries";
const MAIN_CLASS_PATH: &str = "BOOT-INF/classes/com/github/_1c_syntax/bsl/languageserver/MainApplication.class";
const CENTRAL_DIRECTORY_SIGNATURE: u32 = 0x0201_4b50;
const LOCAL_FILE_HEADER_SIGNATURE: u32 = 0x0403_4b50;
const END_OF_CENTRAL_DIRECTORY_SIGNATURE: u32 = 0x0605_4b50;
const MAX_MIN_JAVA_VERSION: u32 = 17;
const CLASS_FILE_MAGIC: [u8; 4] = [0xCA, 0xFE, 0xBA, 0xBE];

struct BslExtension {
    cached_java_version: Option<(String, u32)>,
}

impl BslExtension {
    fn new() -> Self {
        Self {
            cached_java_version: None,
        }
    }

    fn command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        if let Some(binary) = self.user_binary_settings(language_server_id, worktree)? {
            return Ok(binary);
        }

        let java = worktree
            .which("java")
            .ok_or_else(|| java_not_found_message())?;

        let jar = match self.existing_jar_path(worktree)? {
            Some(jar) => jar,
            None => self.download_latest_jar(language_server_id)?,
        };

        let required_java_version = self.required_java_version(language_server_id, &jar)?;
        let installed_java_version = self.detected_java_version(language_server_id, &java)?;

        if installed_java_version < required_java_version {
            return Err(format!(
                "{} requires Java {} or newer, but the Java runtime at `{}` is version {}. \
                 Please install Java {} (e.g. Temurin OpenJDK) and make sure it is on your `PATH`.",
                LANGUAGE_SERVER_NAME,
                required_java_version,
                java,
                installed_java_version,
                max_minimum_java_version(required_java_version),
            ));
        }

        let mut args = self.heap_options(worktree);
        args.push("-jar".to_string());
        args.push(jar.to_string_lossy().into_owned());

        Ok(Command {
            command: java,
            args,
            env: Vec::new(),
        })
    }

    fn user_binary_settings(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<Command>> {
        let settings = LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree).ok();
        let Some(binary) = settings.and_then(|settings| settings.binary) else {
            return Ok(None);
        };
        let Some(command) = binary.path else {
            return Ok(None);
        };
        let env = binary
            .env
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<(String, String)>>();
        Ok(Some(Command {
            command,
            args: binary.arguments.unwrap_or_default(),
            env,
        }))
    }

    fn existing_jar_path(&self, worktree: &Worktree) -> Result<Option<PathBuf>> {
        let shell_env = worktree.shell_env();

        if let Some(path) = env_var(&shell_env, "BSL_LANGUAGE_SERVER") {
            if let Some(path) = self.jar_path_if_exists(path) {
                return Ok(Some(path));
            }
        }

        let mut candidates = Vec::new();

        if let Some(home) = env_var(&shell_env, "HOME") {
            candidates.push(PathBuf::from(home).join(".local/lib"));
        }
        if let Some(data_home) = env_var(&shell_env, "XDG_DATA_HOME") {
            candidates.push(PathBuf::from(data_home).join("lib"));
        }

        for base in candidates {
            let path = base.join(LANGUAGE_SERVER_NAME).join(BSL_JAR_FILENAME);
            if let Some(path) = self.jar_path_if_exists(path) {
                return Ok(Some(path));
            }
        }

        if let Ok(current_dir) = env::current_dir() {
            let path = current_dir
                .join(MANAGED_JAR_DIRECTORY)
                .join(BSL_JAR_FILENAME);
            if path.is_file() {
                return Ok(Some(path));
            }
        }

        Ok(None)
    }

    fn jar_path_if_exists(&self, path: impl AsRef<Path>) -> Option<PathBuf> {
        let path = path.as_ref();
        path.is_file().then(|| path.to_path_buf())
    }

    fn download_latest_jar(
        &self,
        language_server_id: &LanguageServerId,
    ) -> Result<PathBuf> {
        zed::set_language_server_installation_status(
            language_server_id,
            &LanguageServerInstallationStatus::CheckingForUpdate,
        );

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
            .find(|asset| asset.name == BSL_JAR_FILENAME)
            .ok_or_else(|| {
                format!(
                    "no `{}` asset found in release {} of {}",
                    BSL_JAR_FILENAME, release.version, BSL_REPOSITORY
                )
            })?;

        fs::create_dir_all(MANAGED_JAR_DIRECTORY).map_err(|error| {
            format!("failed to create {} directory: {}", MANAGED_JAR_DIRECTORY, error)
        })?;

        let jar_path = PathBuf::from(MANAGED_JAR_DIRECTORY).join(BSL_JAR_FILENAME);

        zed::set_language_server_installation_status(
            language_server_id,
            &LanguageServerInstallationStatus::Downloading,
        );

        zed::download_file(
            &asset.download_url,
            &jar_path.to_string_lossy(),
            zed::DownloadedFileType::Uncompressed,
        )
        .map_err(|error| {
            zed::set_language_server_installation_status(
                language_server_id,
                &LanguageServerInstallationStatus::Failed(format!(
                    "Failed to download {}: {}",
                    BSL_JAR_FILENAME, error
                )),
            );
            format!("failed to download {}: {}", BSL_JAR_FILENAME, error)
        })?;

        Ok(jar_path)
    }

    fn heap_options(&self, worktree: &Worktree) -> Vec<String> {
        let shell_env = worktree.shell_env();
        match env_var(&shell_env, "BSL_LANGUAGE_SERVER_JAVA_OPTS") {
            Some(opts) => opts.split_whitespace().map(str::to_string).collect(),
            None => vec!["-Xmx4g".to_string()],
        }
    }

    fn required_java_version(
        &self,
        language_server_id: &LanguageServerId,
        jar_path: &Path,
    ) -> Result<u32> {
        match bytecode_major_version(jar_path)? {
            Some(major) if major >= 49 => return Ok(class_file_version(major)),
            _ => {}
        }

        match build_jdk_spec(jar_path)? {
            Some(version) if version >= MAX_MIN_JAVA_VERSION => Ok(version),
            _ => {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &LanguageServerInstallationStatus::Failed(
                        "Failed to determine the required Java version from the downloaded jar"
                            .to_string(),
                    ),
                );
                Ok(MAX_MIN_JAVA_VERSION)
            }
        }
    }

    fn detected_java_version(
        &mut self,
        language_server_id: &LanguageServerId,
        java: &str,
    ) -> Result<u32> {
        if let Some((path, version)) = &self.cached_java_version {
            if path == java {
                return Ok(*version);
            }
        }

        let output = ProcessCommand::new(java)
            .args(["-XshowSettings:properties", "-version"])
            .output()
            .map_err(|error| {
                format!("failed to run `{} -version`: {}", java, error)
            })?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let version = format!("{}\n{}", stderr, stdout)
            .lines()
            .find_map(|line| {
                if line.contains("java.specification.version") {
                    line.split('=').nth(1).map(str::trim).and_then(parse_java_spec_version)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &LanguageServerInstallationStatus::Failed(
                        format!(
                            "Failed to determine the installed Java version from `{}`",
                            java
                        ),
                    ),
                );
                format!(
                    "failed to determine the installed Java version from `java` at {}",
                    java
                )
            })?;

        self.cached_java_version = Some((java.to_string(), version));
        Ok(version)
    }
}

impl zed::Extension for BslExtension {
    fn new() -> Self {
        Self::new()
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        self.command(language_server_id, worktree)
    }
}

zed::register_extension!(BslExtension);

fn env_var(env_vars: &zed::EnvVars, key: &str) -> Option<String> {
    env_vars
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.clone())
}

fn java_not_found_message() -> String {
    format!(
        "A Java runtime is required to run the official {} (a Java application), but `java` was not found on your PATH. \
         Install Java 21 or newer (e.g. Temurin OpenJDK: https://adoptium.net) and try again.",
        LANGUAGE_SERVER_NAME
    )
}

fn max_minimum_java_version(required: u32) -> u32 {
    required.max(MAX_MIN_JAVA_VERSION)
}

/// Maps a JVM bytecode class-file major version to the corresponding Java
/// version number (e.g. 65 -> 21, 61 -> 17).
fn class_file_version(major: u32) -> u32 {
    major - 44
}

/// Parses a `java.specification.version` value such as `21`, `17.0.1` or the
/// legacy `1.8` into an integer Java version (8, 17, 21, ...).
fn parse_java_spec_version(value: &str) -> Option<u32> {
    let value = value.trim();
    if let Some(value) = value.strip_prefix("1.") {
        return value.split('.').next().and_then(|part| part.parse().ok());
    }
    value.split('.').next().and_then(|part| part.parse().ok())
}

struct ZipEntry {
    method: u16,
    compressed_size: u32,
    local_header_offset: u32,
}

fn bytecode_major_version(jar_path: &Path) -> Result<Option<u32>> {
    let entry = find_zip_entry(jar_path, MAIN_CLASS_PATH)?;
    let Some(entry) = entry else {
        return Ok(None);
    };
    let data = zip_entry_contents(jar_path, entry)?;
    if data.len() < 8 || data[..4] != CLASS_FILE_MAGIC {
        return Ok(None);
    }
    Ok(Some(((data[6] as u32) << 8) | data[7] as u32))
}

fn build_jdk_spec(jar_path: &Path) -> Result<Option<u32>> {
    let entry = find_zip_entry(jar_path, "META-INF/MANIFEST.MF")?;
    let Some(entry) = entry else {
        return Ok(None);
    };
    let data = zip_entry_contents(jar_path, entry)?;
    let manifest = String::from_utf8_lossy(&data);
    Ok(manifest.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.trim() == "Build-Jdk-Spec" {
            value.trim().parse().ok()
        } else {
            None
        }
    }))
}

fn find_zip_entry(jar_path: &Path, name: &str) -> Result<Option<ZipEntry>> {
    let mut file = fs::File::open(jar_path)
        .map_err(|error| format!("failed to open jar at {}: {}", jar_path.display(), error))?;

    let Some(end_of_central_directory) = find_end_of_central_directory(&mut file)? else {
        return Err(format!(
            "invalid jar at {}: end of central directory record not found",
            jar_path.display()
        ));
    };

    let entry_count = read_central_directory_entry_count(&end_of_central_directory);
    let central_directory_size = read_central_directory_size(&end_of_central_directory);
    let central_directory_offset = read_central_directory_offset(&end_of_central_directory);

    let mut central_directory = vec![0u8; central_directory_size];
    file.seek(SeekFrom::Start(central_directory_offset as u64))
        .map_err(io_err)?;
    file.read_exact(&mut central_directory).map_err(io_err)?;

    let mut offset = 0;
    for _ in 0..entry_count {
        if offset + 46 > central_directory.len()
            || read_u32(&central_directory, offset) != CENTRAL_DIRECTORY_SIGNATURE
        {
            return Err(format!(
                "invalid jar at {}: corrupt central directory",
                jar_path.display()
            ));
        }

        let method = read_u16(&central_directory, offset + 10);
        let compressed_size = read_u32(&central_directory, offset + 20);
        let name_length = read_u16(&central_directory, offset + 28) as usize;
        let extra_length = read_u16(&central_directory, offset + 30) as usize;
        let comment_length = read_u16(&central_directory, offset + 32) as usize;
        let local_header_offset = read_u32(&central_directory, offset + 42);

        let name_end = offset + 46 + name_length;
        if name_end <= central_directory.len()
            && &central_directory[offset + 46..name_end] == name.as_bytes()
        {
            return Ok(Some(ZipEntry {
                method,
                compressed_size,
                local_header_offset,
            }));
        }

        offset = name_end + extra_length + comment_length;
    }

    Ok(None)
}

fn zip_entry_contents(jar_path: &Path, entry: ZipEntry) -> Result<Vec<u8>> {
    let mut file = fs::File::open(jar_path)
        .map_err(|error| format!("failed to open jar at {}: {}", jar_path.display(), error))?;

    let mut local_header = [0u8; 30];
    file.seek(SeekFrom::Start(entry.local_header_offset as u64))
        .map_err(io_err)?;
    file.read_exact(&mut local_header).map_err(io_err)?;

    if read_u32(&local_header, 0) != LOCAL_FILE_HEADER_SIGNATURE {
        return Err(format!(
            "invalid jar at {}: corrupt local file header",
            jar_path.display()
        ));
    }

    let name_length = read_u16(&local_header, 26) as u64;
    let extra_length = read_u16(&local_header, 28) as u64;
    let data_offset = entry.local_header_offset as u64 + 30 + name_length + extra_length;

    let mut compressed = vec![0u8; entry.compressed_size as usize];
    file.seek(SeekFrom::Start(data_offset)).map_err(io_err)?;
    file.read_exact(&mut compressed).map_err(io_err)?;

    match entry.method {
        0 => Ok(compressed),
        8 => inflate::decompress_to_vec(&compressed)
            .map_err(|error| format!("failed to decompress zip entry: {}", error)),
        other => Err(format!("unsupported zip compression method: {}", other)),
    }
}

struct EndOfCentralDirectory {
    record: Vec<u8>,
}

fn find_end_of_central_directory(file: &mut fs::File) -> Result<Option<EndOfCentralDirectory>> {
    let file_size = file
        .metadata()
        .map_err(|error| format!("failed to read jar metadata: {}", error))?
        .len();

    let tail_size = file_size.min(65_557);
    let mut tail = vec![0u8; tail_size as usize];
    file.seek(SeekFrom::End(-(tail_size as i64)))
        .map_err(io_err)?;
    file.read_exact(&mut tail).map_err(io_err)?;

    for index in (0..tail.len().saturating_sub(4)).rev() {
        if read_u32(&tail, index) == END_OF_CENTRAL_DIRECTORY_SIGNATURE {
            return Ok(Some(EndOfCentralDirectory {
                record: tail[index..index + 22].to_vec(),
            }));
        }
    }

    Ok(None)
}

fn read_central_directory_entry_count(eocd: &EndOfCentralDirectory) -> usize {
    read_u16(&eocd.record, 10) as usize
}

fn read_central_directory_size(eocd: &EndOfCentralDirectory) -> usize {
    read_u32(&eocd.record, 12) as usize
}

fn read_central_directory_offset(eocd: &EndOfCentralDirectory) -> usize {
    read_u32(&eocd.record, 16) as usize
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn io_err(error: std::io::Error) -> String {
    format!("I/O error: {}", error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_class_file_major_to_java_version() {
        assert_eq!(class_file_version(61), 17);
        assert_eq!(class_file_version(65), 21);
        assert_eq!(class_file_version(69), 25);
    }

    #[test]
    fn parses_java_specification_versions() {
        assert_eq!(parse_java_spec_version("21"), Some(21));
        assert_eq!(parse_java_spec_version("21.0.2"), Some(21));
        assert_eq!(parse_java_spec_version("17.0.1"), Some(17));
        assert_eq!(parse_java_spec_version("1.8"), Some(8));
        assert_eq!(parse_java_spec_version("25"), Some(25));
        assert_eq!(parse_java_spec_version("unknown"), None);
    }

    #[test]
    fn reads_java_version_from_local_jar() {
        let jar_path = PathBuf::from(env!("HOME"))
            .join(".local/lib/bsl-language-server/bsl-language-server.jar");
        if !jar_path.is_file() {
            return;
        }

        let major = bytecode_major_version(&jar_path).unwrap().unwrap();
        assert!(major >= 65, "main class should be compiled for Java 21+ (major >= 65)");

        let build_jdk_spec = build_jdk_spec(&jar_path).unwrap();
        assert_eq!(build_jdk_spec, Some(21));
    }
}