use eframe::egui;
use std::process::Command;
use std::sync::mpsc;

#[derive(Clone, Debug)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub release_url: String,
}

#[derive(Clone, Debug)]
pub enum UpdateCheckState {
    Idle,
    Checking,
    UpToDate,
    Available(UpdateInfo),
    Failed,
}

fn parse_semver_tuple(s: &str) -> Option<(u32, u32, u32)> {
    let cleaned = s.trim().trim_start_matches(['v', 'V']);
    let core = cleaned.split(['-', '+']).next().unwrap_or(cleaned);
    let mut parts = core.split('.');
    let maj = parts.next()?.parse::<u32>().ok()?;
    let min = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    let pat = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    Some((maj, min, pat))
}

pub fn is_newer_version(current: &str, candidate: &str) -> bool {
    match (parse_semver_tuple(current), parse_semver_tuple(candidate)) {
        (Some(cur), Some(cand)) => cand > cur,
        _ => false,
    }
}

fn github_api_latest_url(repo_url: &str) -> Option<String> {
    let trimmed = repo_url.trim().trim_end_matches('/');
    let idx = trimmed.find("github.com/")?;
    let slug = &trimmed[idx + "github.com/".len()..];
    let mut parts = slug.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        owner, repo
    ))
}

pub fn start_update_check(
    current_version: &'static str,
    repo_url: &'static str,
    ctx: egui::Context,
) -> mpsc::Receiver<Result<Option<UpdateInfo>, String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = check_latest_blocking(current_version, repo_url);
        let _ = tx.send(result);
        ctx.request_repaint();
    });
    rx
}

fn check_latest_blocking(
    current_version: &str,
    repo_url: &str,
) -> Result<Option<UpdateInfo>, String> {
    let api_url = github_api_latest_url(repo_url).ok_or_else(|| "Invalid repo URL".to_string())?;
    let mut cmd = Command::new("curl");
    cmd.args([
        "-fsSL",
        "--max-time",
        "6",
        "-H",
        "User-Agent: ReStem-Update-Checker",
        &api_url,
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }

    let output = cmd.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!("curl exited with {}", output.status));
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let tag_name = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing tag_name".to_string())?;
    let release_url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}/releases/latest", repo_url.trim_end_matches('/')));

    if is_newer_version(current_version, tag_name) {
        Ok(Some(UpdateInfo {
            latest_version: tag_name.trim_start_matches(['v', 'V']).to_string(),
            release_url,
        }))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison_and_api_url() {
        assert!(is_newer_version("1.5.0", "v1.6.0"));
        assert!(is_newer_version("1.5.0", "v1.5.1"));
        assert!(is_newer_version("1.5.0", "v2.0.0"));
        assert!(!is_newer_version("1.5.0", "v1.5.0"));
        assert!(!is_newer_version("1.5.0", "v1.4.0"));
        assert_eq!(
            github_api_latest_url("https://github.com/pirotehnikng-cloud/ReStem").as_deref(),
            Some("https://api.github.com/repos/pirotehnikng-cloud/ReStem/releases/latest")
        );
    }
}
