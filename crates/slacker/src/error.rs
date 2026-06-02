use std::fmt::{self, Display, Formatter};
use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;

/// Error returned by argument parsing, download, or conversion.
#[derive(Debug)]
pub struct Error {
    kind: Kind,
}

#[derive(Debug)]
enum Kind {
    BadGiphyUrl(String),
    CommandFailed { action: &'static str, status: ExitStatus, stderr: String },
    Help,
    Io { action: &'static str, path: PathBuf, source: io::Error },
    MissingUrl,
    MissingValue(&'static str),
    NoCandidate { max_bytes: u64 },
    TooManyUrls,
    UnknownArg(String),
}

impl Error {
    pub(super) fn bad_giphy_url(url: String) -> Self {
        Self { kind: Kind::BadGiphyUrl(url) }
    }

    pub(super) fn command_failed(action: &'static str, status: ExitStatus, stderr: String) -> Self {
        Self { kind: Kind::CommandFailed { action, status, stderr } }
    }

    pub(super) fn help() -> Self {
        Self { kind: Kind::Help }
    }

    pub(super) fn io(action: &'static str, path: PathBuf, source: io::Error) -> Self {
        Self { kind: Kind::Io { action, path, source } }
    }

    pub(super) fn missing_url() -> Self {
        Self { kind: Kind::MissingUrl }
    }

    pub(super) fn missing_value(flag: &'static str) -> Self {
        Self { kind: Kind::MissingValue(flag) }
    }

    pub(super) fn no_candidate(max_bytes: u64) -> Self {
        Self { kind: Kind::NoCandidate { max_bytes } }
    }

    pub(super) fn too_many_urls() -> Self {
        Self { kind: Kind::TooManyUrls }
    }

    pub(super) fn unknown_arg(arg: String) -> Self {
        Self { kind: Kind::UnknownArg(arg) }
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match &self.kind {
            Kind::BadGiphyUrl(url) => write!(formatter, "could not find a GIPHY id in {url}"),
            Kind::CommandFailed { action, status, stderr } => {
                write!(formatter, "{action} failed with {status}: {}", stderr.trim())
            }
            Kind::Help => write!(formatter, "{}", usage()),
            Kind::Io { action, path, source } => {
                write!(formatter, "{action} {}: {source}", path.display())
            }
            Kind::MissingUrl => write!(formatter, "missing GIPHY url\n{}", usage()),
            Kind::MissingValue(flag) => write!(formatter, "{flag} needs a value"),
            Kind::NoCandidate { max_bytes } => {
                write!(formatter, "could not make a GIF under {max_bytes} bytes")
            }
            Kind::TooManyUrls => write!(formatter, "pass exactly one GIPHY url"),
            Kind::UnknownArg(arg) => write!(formatter, "unknown argument {arg}"),
        }
    }
}

impl std::error::Error for Error {}

fn usage() -> String {
    format!("usage: {} <giphy-url> [--out-dir DIR] [--name NAME] [--json]", command_name())
}

fn command_name() -> String {
    std::env::args_os()
        .next()
        .and_then(|value| PathBuf::from(value).file_name().map(std::ffi::OsString::from))
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| String::from("slacker"))
}
