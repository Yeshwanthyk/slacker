use std::path::PathBuf;

use crate::error::Error;

#[derive(Debug)]
/// Parsed command-line configuration.
pub struct Config {
    pub(super) json: bool,
    pub(super) name: Option<String>,
    pub(super) out_dir: PathBuf,
    pub(super) url: String,
}

/// Parses `slacker` command-line arguments.
///
/// # Errors
///
/// Returns an error when the URL is missing, a flag is unknown, or a flag value
/// is missing.
pub fn parse(args: impl Iterator<Item = String>) -> Result<Config, Error> {
    let mut out_dir = PathBuf::from("/tmp");
    let mut json = false;
    let mut name = None;
    let mut url = None;
    let mut iter = args;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => return Err(Error::help()),
            "--json" => json = true,
            "--out-dir" | "-o" => out_dir = value_path(&mut iter, "--out-dir")?,
            "--name" | "-n" => name = Some(value_string(&mut iter, "--name")?),
            "make" if url.is_none() => {}
            _ if arg.starts_with('-') => return Err(Error::unknown_arg(arg)),
            _ if url.is_none() => url = Some(arg),
            _ => return Err(Error::too_many_urls()),
        }
    }

    let Some(url) = url else {
        return Err(Error::missing_url());
    };

    Ok(Config { json, name, out_dir, url })
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
        assert_eq!(config.url, "https://giphy.com/gifs/name-ID123abc");
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
}
