//! Optional upload of a produced emoji to a Slack workspace.
//!
//! Slack exposes no official public method for adding a custom emoji, so this
//! posts to the workspace's internal `emoji.add` endpoint with `curl`, matching
//! how community emoji tooling works. Credentials come from the environment:
//!
//! - `SLACK_TOKEN` (required): an `xoxc`/`xoxs`/`xoxp` token with emoji rights.
//! - `SLACK_TEAM` (or `--team`): the workspace subdomain in `<team>.slack.com`.
//! - `SLACK_COOKIE` (optional): the `d` cookie value, required for `xoxc` tokens.

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::Error;

/// Resolved Slack upload credentials and target workspace.
#[derive(Debug)]
pub struct Target {
    token: String,
    team: String,
    cookie: Option<String>,
}

impl Target {
    /// Resolves an upload target from `--team` and the environment.
    ///
    /// # Errors
    ///
    /// Returns an error when `SLACK_TOKEN` or the workspace subdomain is absent.
    pub fn resolve(team: Option<&str>) -> Result<Self, Error> {
        let Some(token) = env_var("SLACK_TOKEN") else {
            return Err(Error::missing_slack_config("SLACK_TOKEN"));
        };
        let team = match team.map(str::to_owned).or_else(|| env_var("SLACK_TEAM")) {
            Some(value) => value,
            None => return Err(Error::missing_slack_config("--team or SLACK_TEAM")),
        };
        Ok(Self { token, team, cookie: env_var("SLACK_COOKIE") })
    }

    fn endpoint(&self) -> String {
        format!("https://{}.slack.com/api/emoji.add", self.team)
    }
}

/// Uploads `image` as a custom emoji named `name`.
///
/// The token and cookie are passed through a private `curl --config` file rather
/// than argv, so they never appear in the process table (`ps`).
///
/// # Errors
///
/// Returns an error when `curl` fails to run or Slack rejects the upload.
pub fn send(target: &Target, image: &Path, name: &str) -> Result<(), Error> {
    let secrets = SecretsFile::write(target, image)?;

    let mut command = Command::new("curl");
    command
        .arg("--config")
        .arg(secrets.path())
        .arg("--silent")
        .arg("--show-error")
        .arg("--form")
        .arg("mode=data")
        .arg("--form")
        .arg(format!("name={name}"))
        .arg("--form")
        .arg(format!("image=@{}", image.display()))
        .arg(target.endpoint());

    let outcome = command.output();
    // The config file holds the token; remove it regardless of the result.
    secrets.cleanup();

    let output =
        outcome.map_err(|source| Error::io("upload to Slack", PathBuf::from("curl"), source))?;
    if !output.status.success() {
        return Err(Error::command_failed(
            "upload to Slack",
            output.status,
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    parse_response(&String::from_utf8_lossy(&output.stdout))
}

/// A `curl --config` file holding the token and cookie, written with `0600`
/// permissions and removed after the request.
#[derive(Debug)]
struct SecretsFile {
    path: PathBuf,
}

impl SecretsFile {
    fn write(target: &Target, image: &Path) -> Result<Self, Error> {
        let directory = image.parent().unwrap_or_else(|| Path::new("."));
        let path = directory.join(format!(".slacker-curl-{}.cfg", std::process::id()));

        let mut body = format!("form = \"token={}\"\n", target.token);
        if let Some(cookie) = &target.cookie {
            body.push_str(&format!("cookie = \"d={cookie}\"\n"));
        }
        write_private(&path, body.as_bytes())?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn cleanup(&self) {
        // Best-effort: a leftover config file must not mask the upload result.
        discard(fs::remove_file(&self.path));
    }
}

fn write_private(path: &Path, body: &[u8]) -> Result<(), Error> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Owner read/write only; the file briefly contains the Slack token.
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|source| Error::io("write upload credentials", path.to_path_buf(), source))?;
    file.write_all(body)
        .map_err(|source| Error::io("write upload credentials", path.to_path_buf(), source))
}

fn discard(_outcome: std::io::Result<()>) {}

fn parse_response(body: &str) -> Result<(), Error> {
    let compact: String = body.chars().filter(|character| !character.is_whitespace()).collect();
    // Slack always returns `ok` as the first field; anchoring to `{"ok":true`
    // avoids a false success from the literal appearing inside an error string.
    if compact.starts_with("{\"ok\":true") {
        return Ok(());
    }
    let detail =
        extract_field(&compact, "\"error\":\"").unwrap_or_else(|| String::from(body.trim()));
    Err(Error::slack_upload(detail))
}

fn extract_field(compact: &str, key: &str) -> Option<String> {
    let after = compact.split(key).nth(1)?;
    let value = after.split('"').next()?;
    if value.is_empty() { None } else { Some(value.to_owned()) }
}

fn env_var(key: &str) -> Option<String> {
    env::var(key).ok().filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::parse_response;

    #[test]
    fn accepts_ok_response() {
        assert!(parse_response(r#"{"ok":true}"#).is_ok());
    }

    #[test]
    fn surfaces_slack_error() {
        let error = match parse_response(r#"{"ok":false,"error":"error_name_taken"}"#) {
            Ok(()) => panic!("expected an error"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("error_name_taken"), "got: {error}");
    }

    #[test]
    fn tolerates_whitespace_in_response() {
        assert!(parse_response("{ \"ok\" : true }").is_ok());
    }

    #[test]
    fn rejects_embedded_ok_true_substring() {
        // A failure whose message merely contains the literal must not pass.
        let body = r#"{"ok":false,"error":"set \"ok\":true to proceed"}"#;
        assert!(parse_response(body).is_err());
    }

    #[test]
    fn upload_without_token_reports_missing_config() {
        // Guards the env-resolution error path without touching the network.
        if std::env::var("SLACK_TOKEN").is_ok() {
            return;
        }
        let error = match super::Target::resolve(Some("acme")) {
            Ok(_) => panic!("expected missing-config error"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("SLACK_TOKEN"), "got: {error}");
    }
}
