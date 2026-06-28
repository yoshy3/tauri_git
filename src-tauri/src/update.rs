use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/yoshy3/tauri_git/releases/latest";
const USER_AGENT: &str = "tauri-git-update-checker";
static PENDING_EXIT_INSTALLER: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ReleaseAsset {
    pub(crate) name: String,
    pub(crate) browser_download_url: String,
    pub(crate) size: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct UpdateInfo {
    pub(crate) version: String,
    pub(crate) tag_name: String,
    pub(crate) release_name: String,
    pub(crate) html_url: String,
    pub(crate) asset: ReleaseAsset,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    html_url: String,
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

pub(crate) async fn check_latest_release() -> Result<Option<UpdateInfo>, String> {
    let client = reqwest::Client::new();
    let response = client
        .get(GITHUB_LATEST_RELEASE_URL)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .map_err(|error| format!("最新版の確認に失敗しました / Failed to check for updates: {error}"))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Err(format!(
            "最新版の確認に失敗しました / Failed to check for updates: HTTP {}",
            response.status()
        ));
    }

    let release = response
        .json::<GitHubRelease>()
        .await
        .map_err(|error| format!("Release 情報の解析に失敗しました / Failed to parse release data: {error}"))?;

    let latest_version = normalize_version(&release.tag_name);
    if !is_newer_version(&latest_version, env!("CARGO_PKG_VERSION")) {
        return Ok(None);
    }

    let asset = select_platform_asset(&release.assets).ok_or_else(|| {
        "この環境に適合するインストーラが Release に見つかりません / No matching installer asset was found for this platform"
            .to_string()
    })?;

    Ok(Some(UpdateInfo {
        version: latest_version,
        tag_name: release.tag_name,
        release_name: release.name.unwrap_or_default(),
        html_url: release.html_url,
        asset: ReleaseAsset {
            name: asset.name.clone(),
            browser_download_url: asset.browser_download_url.clone(),
            size: asset.size,
        },
    }))
}

pub(crate) async fn download_and_run_installer(
    app_handle: AppHandle,
    update: UpdateInfo,
) -> Result<(), String> {
    let installer_path = download_installer(app_handle, &update).await?;
    run_installer(&installer_path)
}

pub(crate) async fn download_installer_for_exit(
    app_handle: AppHandle,
    update: UpdateInfo,
) -> Result<(), String> {
    let installer_path = download_installer(app_handle, &update).await?;
    let pending = PENDING_EXIT_INSTALLER.get_or_init(|| Mutex::new(None));
    let mut pending = pending.lock().map_err(|error| {
        format!("終了時アップデートの予約に失敗しました / Failed to schedule update on exit: {error}")
    })?;
    *pending = Some(installer_path);
    Ok(())
}

pub(crate) fn run_pending_exit_installer() {
    let Some(pending) = PENDING_EXIT_INSTALLER.get() else {
        return;
    };
    let Ok(mut pending) = pending.lock() else {
        return;
    };
    let Some(installer_path) = pending.take() else {
        return;
    };
    let _ = run_installer(&installer_path);
}

async fn download_installer(app_handle: AppHandle, update: &UpdateInfo) -> Result<PathBuf, String> {
    let client = reqwest::Client::new();
    let response = client
        .get(&update.asset.browser_download_url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .map_err(|error| format!("インストーラのダウンロードに失敗しました / Failed to download installer: {error}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "インストーラのダウンロードに失敗しました / Failed to download installer: HTTP {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("インストーラの読み込みに失敗しました / Failed to read installer data: {error}"))?;
    let installer_path = installer_download_path(&app_handle, update)?;
    if let Some(parent) = installer_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!("保存先フォルダを作成できません / Failed to create download folder: {error}")
        })?;
    }
    fs::write(&installer_path, bytes).map_err(|error| {
        format!("インストーラを保存できません / Failed to save installer: {error}")
    })?;

    Ok(installer_path)
}

fn installer_download_path(app_handle: &AppHandle, update: &UpdateInfo) -> Result<PathBuf, String> {
    let base_dir = app_handle.path().app_cache_dir().map_err(|error| {
        format!("保存先フォルダを取得できません / Failed to resolve cache folder: {error}")
    })?;
    Ok(base_dir
        .join("updates")
        .join(&update.version)
        .join(sanitize_file_name(&update.asset.name)))
}

fn run_installer(path: &PathBuf) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let mut command = {
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("msi"))
        {
            let mut command = Command::new("msiexec.exe");
            command.arg("/i").arg(path);
            command
        } else {
            Command::new(path)
        }
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(path);
        command
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("インストーラを起動できません / Failed to run installer: {error}"))
}

fn select_platform_asset(assets: &[GitHubReleaseAsset]) -> Option<&GitHubReleaseAsset> {
    assets
        .iter()
        .filter_map(|asset| {
            let score = platform_asset_score(&asset.name)?;
            Some((score, asset))
        })
        .max_by_key(|(score, _asset)| *score)
        .map(|(_score, asset)| asset)
}

fn platform_asset_score(name: &str) -> Option<u8> {
    let lower_name = name.to_ascii_lowercase();
    if !platform_extension_candidates()
        .iter()
        .any(|extension| lower_name.ends_with(extension))
    {
        return None;
    }

    if other_arch_candidates()
        .iter()
        .any(|candidate| lower_name.contains(candidate))
    {
        return None;
    }

    let mut score = 1;
    if platform_candidates()
        .iter()
        .any(|candidate| lower_name.contains(candidate))
    {
        score += 10;
    }
    if target_arch_candidates()
        .iter()
        .any(|candidate| lower_name.contains(candidate))
    {
        score += 100;
    }
    Some(score)
}

fn platform_candidates() -> &'static [&'static str] {
    #[cfg(target_os = "windows")]
    {
        &["windows", "win"]
    }

    #[cfg(target_os = "macos")]
    {
        &["darwin", "macos", "mac"]
    }

    #[cfg(target_os = "linux")]
    {
        &["linux"]
    }
}

fn target_arch_candidates() -> &'static [&'static str] {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        &["x64", "x86_64", "amd64"]
    }

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        &["arm64", "aarch64"]
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        &["x64", "x86_64", "intel", "amd64"]
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        &["arm64", "aarch64", "apple"]
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        &["x64", "x86_64", "amd64"]
    }

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        &["arm64", "aarch64"]
    }
}

fn other_arch_candidates() -> &'static [&'static str] {
    #[cfg(target_arch = "x86_64")]
    {
        &["arm64", "aarch64"]
    }

    #[cfg(target_arch = "aarch64")]
    {
        &["x64", "x86_64", "amd64", "intel"]
    }
}

fn platform_extension_candidates() -> &'static [&'static str] {
    #[cfg(target_os = "windows")]
    {
        &[".msi", ".exe"]
    }

    #[cfg(target_os = "macos")]
    {
        &[".dmg", ".pkg"]
    }

    #[cfg(target_os = "linux")]
    {
        &[".appimage", ".deb", ".rpm"]
    }
}

fn normalize_version(version: &str) -> String {
    version
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string()
}

fn is_newer_version(candidate: &str, current: &str) -> bool {
    let candidate_parts = version_parts(candidate);
    let current_parts = version_parts(current);
    for index in 0..candidate_parts.len().max(current_parts.len()) {
        let candidate_part = *candidate_parts.get(index).unwrap_or(&0);
        let current_part = *current_parts.get(index).unwrap_or(&0);
        if candidate_part != current_part {
            return candidate_part > current_part;
        }
    }
    false
}

fn version_parts(version: &str) -> Vec<u64> {
    version
        .split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

fn sanitize_file_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => character,
        })
        .collect();
    if sanitized.trim().is_empty() {
        "installer".to_string()
    } else {
        sanitized
    }
}
