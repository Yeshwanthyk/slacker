use std::path::PathBuf;

use crate::error::Error;

#[derive(Debug)]
/// Parsed command-line configuration.
pub struct Config {
    pub(super) json: bool,
    pub(super) name: Option<String>,
    pub(super) out_dir: PathBuf,
    pub(super) input: String,
    pub(super) upload: bool,
    pub(super) team: Option<String>,
    pub(super) max_bytes: u64,
    pub(super) max_frames: u32,
    pub(super) pad: bool,
    pub(super) force: bool,
}

/// Default upper bound on the output GIF size. Slack's hard cap is 128 KiB;
/// this leaves headroom for client-side overhead.
const DEFAULT_MAX_BYTES: u64 = 120_000;

/// Default cap on frames kept from the source clip.
const DEFAULT_MAX_FRAMES: u32 = 50;

/// Parses `slacker` command-line arguments.
///
/// # Errors
///
/// Returns an error when the input is missing, a flag is unknown, or a flag
/// value is missing.
pub fn parse(args: impl Iterator<Item = String>) -> Result<Config, Error> {
    let mut out_dir = PathBuf::from("/tmp");
    let mut json = false;
    let mut name = None;
    let mut input = None;
    let mut upload = false;
    let mut team = None;
    let mut max_bytes = DEFAULT_MAX_BYTES;
    let mut max_frames = DEFAULT_MAX_FRAMES;
    let mut pad = false;
    let mut force = false;
    let mut iter = args;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => return Err(Error::help()),
            "--json" => json = true,
            "--upload" => upload = true,
            "--force" => force = true,
            "--team" => team = Some(value_string(&mut iter, "--team")?),
            "--out-dir" | "-o" => out_dir = value_path(&mut iter, "--out-dir")?,
            "--name" | "-n" => name = Some(value_string(&mut iter, "--name")?),
            "--max-bytes" => max_bytes = value_u64(&mut iter, "--max-bytes")?,
            "--max-frames" => max_frames = value_u32(&mut iter, "--max-frames")?,
            "--fit" => pad = value_fit(&mut iter)?,
            "make" if input.is_none() => {}
            "-" if input.is_none() => input = Some(arg),
            _ if arg.starts_with('-') => return Err(Error::unknown_arg(arg)),
            _ if input.is_none() => input = Some(arg),
            _ => return Err(Error::too_many_inputs()),
        }
    }

    let Some(input) = input else {
        return Err(Error::missing_input());
    };

    Ok(Config { json, name, out_dir, input, upload, team, max_bytes, max_frames, pad, force })
}

fn value_path(
    args: &mut impl Iterator<Item = String>,
    flag: &'static str,
) -> Result<PathBuf, Error> {
    Ok(PathBuf::from(value_string(args, flag)?))
}

fn value_string(
    args: &mut impl Iterator<Item = String>,
    flag: &'static str,
) -> Result<String, Error> {
    let Some(value) = args.next() else {
        return Err(Error::missing_value(flag));
    };
    Ok(value)
}

fn value_u64(args: &mut impl Iterator<Item = String>, flag: &'static str) -> Result<u64, Error> {
    let raw = value_string(args, flag)?;
    match raw.parse::<u64>() {
        Ok(value) if value > 0 => Ok(value),
        Ok(_) | Err(_) => Err(Error::invalid_value(flag, raw)),
    }
}

fn value_u32(args: &mut impl Iterator<Item = String>, flag: &'static str) -> Result<u32, Error> {
    let raw = value_string(args, flag)?;
    match raw.parse::<u32>() {
        Ok(value) if value > 0 => Ok(value),
        Ok(_) | Err(_) => Err(Error::invalid_value(flag, raw)),
    }
}

fn value_fit(args: &mut impl Iterator<Item = String>) -> Result<bool, Error> {
    let raw = value_string(args, "--fit")?;
    match raw.as_str() {
        "crop" => Ok(false),
        "pad" => Ok(true),
        _ => Err(Error::invalid_value("--fit", raw)),
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn defaults_output_to_tmp() {
        let config =
            match parse(["https://giphy.com/gifs/name-ID123abc"].into_iter().map(String::from)) {
                Ok(value) => value,
                Err(error) => panic!("parse failed: {error}"),
            };

        assert_eq!(config.out_dir.to_string_lossy(), "/tmp");
        assert_eq!(config.input, "https://giphy.com/gifs/name-ID123abc");
    }

    #[test]
    fn accepts_name_and_out_dir() {
        let config = match parse(
            ["--name", "wave", "--out-dir", "out", "https://giphy.com/gifs/name-ID123abc"]
                .into_iter()
                .map(String::from),
        ) {
            Ok(value) => value,
            Err(error) => panic!("parse failed: {error}"),
        };

        assert_eq!(config.name.as_deref(), Some("wave"));
        assert_eq!(config.out_dir.to_string_lossy(), "out");
    }

    #[test]
    fn accepts_upload_and_team() {
        let config = match parse(
            ["--upload", "--team", "acme", "https://giphy.com/gifs/name-ID123abc"]
                .into_iter()
                .map(String::from),
        ) {
            Ok(value) => value,
            Err(error) => panic!("parse failed: {error}"),
        };

        assert!(config.upload);
        assert_eq!(config.team.as_deref(), Some("acme"));
    }

    #[test]
    fn defaults_quality_knobs() {
        let config =
            match parse(["https://giphy.com/gifs/name-ID123abc"].into_iter().map(String::from)) {
                Ok(value) => value,
                Err(error) => panic!("parse failed: {error}"),
            };

        assert_eq!(config.max_bytes, super::DEFAULT_MAX_BYTES);
        assert_eq!(config.max_frames, super::DEFAULT_MAX_FRAMES);
        assert!(!config.pad);
        assert!(!config.force);
    }

    #[test]
    fn accepts_fit_and_size_knobs() {
        let config = match parse(
            ["--fit", "pad", "--max-bytes", "131072", "--max-frames", "30", "--force", "x.gif"]
                .into_iter()
                .map(String::from),
        ) {
            Ok(value) => value,
            Err(error) => panic!("parse failed: {error}"),
        };

        assert!(config.pad);
        assert_eq!(config.max_bytes, 131_072);
        assert_eq!(config.max_frames, 30);
        assert!(config.force);
    }

    #[test]
    fn rejects_invalid_fit() {
        let result = parse(["--fit", "squish", "x.gif"].into_iter().map(String::from));
        assert!(result.is_err());
    }

    #[test]
    fn rejects_non_numeric_max_bytes() {
        let result = parse(["--max-bytes", "lots", "x.gif"].into_iter().map(String::from));
        assert!(result.is_err());
    }

    #[test]
    fn rejects_zero_max_frames() {
        let result = parse(["--max-frames", "0", "x.gif"].into_iter().map(String::from));
        assert!(result.is_err());
    }
}
