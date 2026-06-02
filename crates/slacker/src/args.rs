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
}

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
    let mut iter = args;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => return Err(Error::help()),
            "--json" => json = true,
            "--upload" => upload = true,
            "--team" => team = Some(value_string(&mut iter, "--team")?),
            "--out-dir" | "-o" => out_dir = value_path(&mut iter, "--out-dir")?,
            "--name" | "-n" => name = Some(value_string(&mut iter, "--name")?),
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

    Ok(Config { json, name, out_dir, input, upload, team })
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
}
