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
    InvalidValue { flag: &'static str, value: String },
    Io { action: &'static str, path: PathBuf, source: io::Error },
    MissingInput,
    MissingTool(&'static str),
    MissingValue(&'static str),
    NoCandidate { max_bytes: u64 },
    NoMediaSource,
    OutputExists(PathBuf),
    TenorMediaNotFound(String),
    TooManyInputs,
    UnknownArg(String),
    UnsupportedSource(String),
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

    pub(super) fn invalid_value(flag: &'static str, value: String) -> Self {
        Self { kind: Kind::InvalidValue { flag, value } }
    }

    pub(super) fn io(action: &'static str, path: PathBuf, source: io::Error) -> Self {
        Self { kind: Kind::Io { action, path, source } }
    }

    pub(super) fn missing_input() -> Self {
        Self { kind: Kind::MissingInput }
    }

    pub(super) fn missing_tool(tool: &'static str) -> Self {
        Self { kind: Kind::MissingTool(tool) }
    }

    pub(super) fn missing_value(flag: &'static str) -> Self {
        Self { kind: Kind::MissingValue(flag) }
    }

    pub(super) fn no_candidate(max_bytes: u64) -> Self {
        Self { kind: Kind::NoCandidate { max_bytes } }
    }

    pub(super) fn no_media_source() -> Self {
        Self { kind: Kind::NoMediaSource }
    }

    pub(super) fn output_exists(path: PathBuf) -> Self {
        Self { kind: Kind::OutputExists(path) }
    }

    pub(super) fn tenor_media_not_found(page: String) -> Self {
        Self { kind: Kind::TenorMediaNotFound(page) }
    }

    pub(super) fn too_many_inputs() -> Self {
        Self { kind: Kind::TooManyInputs }
    }

    pub(super) fn unknown_arg(arg: String) -> Self {
        Self { kind: Kind::UnknownArg(arg) }
    }

    pub(super) fn unsupported_source(input: String) -> Self {
        Self { kind: Kind::UnsupportedSource(input) }
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
            Kind::InvalidValue { flag, value } => {
                write!(formatter, "{flag} got an invalid value: {value}")
            }
            Kind::Io { action, path, source } => {
                write!(formatter, "{action} {}: {source}", path.display())
            }
            Kind::MissingInput => write!(formatter, "missing input\n{}", usage()),
            Kind::MissingTool(tool) => {
                write!(formatter, "{tool} was not found on PATH; please install it")
            }
            Kind::MissingValue(flag) => write!(formatter, "{flag} needs a value"),
            Kind::NoCandidate { max_bytes } => {
                write!(formatter, "could not make a GIF under {max_bytes} bytes")
            }
            Kind::NoMediaSource => write!(formatter, "no media URL to download"),
            Kind::OutputExists(path) => {
                write!(formatter, "{} already exists; pass --force to overwrite", path.display())
            }
            Kind::TenorMediaNotFound(page) => {
                write!(formatter, "could not find a Tenor media URL on {page}")
            }
            Kind::TooManyInputs => write!(formatter, "pass exactly one input"),
            Kind::UnknownArg(arg) => write!(formatter, "unknown argument {arg}"),
            Kind::UnsupportedSource(input) => write!(
                formatter,
                "unsupported input {input}: expected a GIPHY/Tenor/Imgur link, \
                 a direct media URL, a local file, or - for stdin"
            ),
        }
    }
}

impl std::error::Error for Error {}

fn usage() -> String {
    format!(
        "usage: {} <url|file|-> [--name NAME] [--out-dir DIR] [--fit crop|pad]\n  \
         [--max-bytes N] [--max-frames N] [--force] [--json]",
        command_name()
    )
}

fn command_name() -> String {
    std::env::args_os()
        .next()
        .and_then(|value| PathBuf::from(value).file_name().map(std::ffi::OsString::from))
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| String::from("slacker"))
}
