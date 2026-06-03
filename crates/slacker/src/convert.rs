use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::args::Config;
use crate::error::Error;
use crate::source::{self, Fetch};
use crate::upload::{self, Target};

/// Upper bound on the sanitized emoji name length.
const MAX_NAME_LEN: usize = 64;

const PROFILES: [Profile; 12] = [
    Profile { size: 128, fps: 10, colors: 64 },
    Profile { size: 128, fps: 8, colors: 64 },
    Profile { size: 128, fps: 6, colors: 64 },
    Profile { size: 112, fps: 8, colors: 64 },
    Profile { size: 112, fps: 6, colors: 64 },
    Profile { size: 112, fps: 5, colors: 64 },
    Profile { size: 96, fps: 8, colors: 64 },
    Profile { size: 96, fps: 6, colors: 64 },
    Profile { size: 96, fps: 5, colors: 64 },
    Profile { size: 80, fps: 8, colors: 64 },
    Profile { size: 80, fps: 6, colors: 48 },
    Profile { size: 80, fps: 5, colors: 32 },
];

#[derive(Debug)]
/// Generated Slack emoji GIF.
pub struct Product {
    /// Output file size in bytes.
    pub bytes: u64,
    /// Whether the caller requested JSON output.
    pub json: bool,
    /// Slack-safe emoji name.
    pub name: String,
    /// Output GIF path.
    pub path: PathBuf,
    /// Whether the emoji was uploaded to Slack.
    pub uploaded: bool,
}

#[derive(Clone, Copy, Debug)]
struct Profile {
    size: u16,
    fps: u8,
    colors: u16,
}

/// Caller-tunable encoding limits applied to every profile in the ladder.
#[derive(Clone, Copy, Debug)]
struct Encode {
    max_bytes: u64,
    max_frames: u32,
    pad: bool,
}

/// Resolves the configured input, fetches it, and converts it to a Slack emoji.
///
/// # Errors
///
/// Returns an error when a required tool is missing, the input cannot be
/// resolved, the output already exists without `--force`, external commands
/// fail, filesystem writes fail, or no candidate fits the size cap.
pub fn make(config: &Config) -> Result<Product, Error> {
    ensure_tool("curl", "--version")?;
    ensure_tool("ffmpeg", "-version")?;

    let source = source::resolve(&config.input)?;
    let name = config
        .name
        .as_deref()
        .map(sanitize_name)
        .unwrap_or_else(|| sanitize_name(&source.name_hint));
    // Resolve upload credentials up front so a misconfigured `--upload` fails
    // before any download or conversion work.
    let target = if config.upload { Some(Target::resolve(config.team.as_deref())?) } else { None };
    let output = config.out_dir.join(format!("{name}.gif"));
    if !config.force && output.exists() {
        return Err(Error::output_exists(output));
    }
    let scratch = config.out_dir.join(format!(".slacker-{name}-source"));
    let candidate = config.out_dir.join(format!(".slacker-{name}-candidate.gif"));
    let encode =
        Encode { max_bytes: config.max_bytes, max_frames: config.max_frames, pad: config.pad };

    create_dir(&config.out_dir)?;
    let materialized = match materialize(&source.fetch, &scratch) {
        Ok(value) => value,
        Err(error) => {
            // A partial download may have written `scratch` before failing.
            discard(remove_file(&scratch));
            return Err(error);
        }
    };
    let result =
        convert_first_fit(&materialized.path, &candidate, &output, &name, config.json, encode);
    if materialized.temporary {
        discard(remove_file(&scratch));
    }

    let mut product = result?;
    if let Some(target) = target {
        upload::send(&target, &product.path, &product.name)?;
        product.uploaded = true;
    }
    Ok(product)
}

fn ensure_tool(tool: &'static str, version_flag: &str) -> Result<(), Error> {
    match Command::new(tool).arg(version_flag).output() {
        Ok(_) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Err(Error::missing_tool(tool)),
        Err(source) => Err(Error::io("check tool", PathBuf::from(tool), source)),
    }
}

/// Drops a best-effort cleanup result; a failed temp-file removal must not mask
/// the real conversion outcome.
fn discard(_outcome: Result<(), Error>) {}

/// The on-disk media to convert and whether it is a scratch file we own.
#[derive(Debug)]
struct Materialized {
    path: PathBuf,
    temporary: bool,
}

fn materialize(fetch: &Fetch, scratch: &Path) -> Result<Materialized, Error> {
    match fetch {
        Fetch::Url(urls) => {
            download_first(urls, scratch)?;
            Ok(Materialized { path: scratch.to_path_buf(), temporary: true })
        }
        Fetch::TenorPage(page) => {
            let html = fetch_text(page)?;
            let candidates = source::extract_tenor_media(&html);
            if candidates.is_empty() {
                return Err(Error::tenor_media_not_found(page.clone()));
            }
            download_first(&candidates, scratch)?;
            Ok(Materialized { path: scratch.to_path_buf(), temporary: true })
        }
        Fetch::File(path) => Ok(Materialized { path: path.clone(), temporary: false }),
        Fetch::Stdin => {
            read_stdin(scratch)?;
            Ok(Materialized { path: scratch.to_path_buf(), temporary: true })
        }
    }
}

fn convert_first_fit(
    source: &Path,
    candidate: &Path,
    output: &Path,
    name: &str,
    json: bool,
    encode: Encode,
) -> Result<Product, Error> {
    for profile in PROFILES {
        convert(source, candidate, profile, encode)?;
        let bytes = file_len(candidate)?;
        if bytes <= encode.max_bytes {
            rename(candidate, output)?;
            return Ok(Product {
                bytes,
                json,
                name: name.to_owned(),
                path: output.to_path_buf(),
                uploaded: false,
            });
        }
        remove_file(candidate)?;
    }
    Err(Error::no_candidate(encode.max_bytes))
}

fn download_first(urls: &[String], path: &Path) -> Result<(), Error> {
    let mut last = None;
    for url in urls {
        match download(url, path) {
            Ok(()) => return Ok(()),
            Err(error) => last = Some(error),
        }
    }
    Err(last.unwrap_or_else(Error::no_media_source))
}

fn download(url: &str, path: &Path) -> Result<(), Error> {
    run(
        Command::new("curl")
            .arg("--fail")
            .arg("--location")
            .arg("--silent")
            .arg("--show-error")
            .arg(url)
            .arg("-o")
            .arg(path),
        "download media",
    )
}

fn fetch_text(url: &str) -> Result<String, Error> {
    let output = Command::new("curl")
        .arg("--fail")
        .arg("--location")
        .arg("--silent")
        .arg("--show-error")
        .arg(url)
        .output()
        .map_err(|source| Error::io("fetch page", PathBuf::from("curl"), source))?;
    if !output.status.success() {
        return Err(Error::command_failed(
            "fetch page",
            output.status,
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn read_stdin(path: &Path) -> Result<(), Error> {
    let mut buffer = Vec::new();
    io::stdin()
        .read_to_end(&mut buffer)
        .map_err(|source| Error::io("read standard input", path.to_path_buf(), source))?;
    fs::write(path, buffer)
        .map_err(|source| Error::io("write source file", path.to_path_buf(), source))
}

fn convert(source: &Path, output: &Path, profile: Profile, encode: Encode) -> Result<(), Error> {
    let size = profile.size;
    let geometry = if encode.pad {
        // Preserve the whole frame: fit inside the square, then pad with a
        // transparent border so nothing is cropped away.
        format!(
            "scale={size}:{size}:force_original_aspect_ratio=decrease:flags=lanczos,\
             format=rgba,pad={size}:{size}:(ow-iw)/2:(oh-ih)/2:color=#00000000"
        )
    } else {
        // Center-crop to a square, then scale.
        format!("crop=min(iw\\,ih):min(iw\\,ih),scale={size}:{size}:flags=lanczos")
    };
    let filter = format!(
        "fps={fps},trim=end_frame={frames},setpts=PTS-STARTPTS,{geometry},\
         split[s0][s1];[s0]palettegen=max_colors={colors}:stats_mode=diff[p];\
         [s1][p]paletteuse=dither=bayer:bayer_scale=3:diff_mode=rectangle",
        fps = profile.fps,
        frames = encode.max_frames,
        colors = profile.colors
    );

    run(
        Command::new("ffmpeg")
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-y")
            .arg("-i")
            .arg(source)
            .arg("-filter_complex")
            .arg(filter)
            .arg(output),
        "convert GIF",
    )
}

fn run(command: &mut Command, action: &'static str) -> Result<(), Error> {
    let output = command.output().map_err(|source| {
        Error::io(action, PathBuf::from(command.get_program().to_os_string()), source)
    })?;
    if output.status.success() {
        return Ok(());
    }

    Err(Error::command_failed(
        action,
        output.status,
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

fn sanitize_name(id: &str) -> String {
    let mut name = String::new();
    for char in id.chars() {
        if name.len() >= MAX_NAME_LEN {
            break;
        }
        if char.is_ascii_alphanumeric() {
            name.push(char.to_ascii_lowercase());
        } else if char == '-' || char == '_' || char.is_ascii_whitespace() {
            name.push('_');
        }
    }

    let trimmed = name.trim_matches('_');
    if trimmed.is_empty() { String::from("emoji") } else { trimmed.to_owned() }
}

fn create_dir(path: &Path) -> Result<(), Error> {
    fs::create_dir_all(path)
        .map_err(|source| Error::io("create output directory", path.to_path_buf(), source))
}

fn file_len(path: &Path) -> Result<u64, Error> {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|source| Error::io("stat file", path.to_path_buf(), source))
}

fn rename(from: &Path, to: &Path) -> Result<(), Error> {
    fs::rename(from, to).map_err(|source| Error::io("write output file", to.to_path_buf(), source))
}

fn remove_file(path: &Path) -> Result<(), Error> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::io("remove temp file", path.to_path_buf(), source)),
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_NAME_LEN, sanitize_name};

    #[test]
    fn sanitizes_and_lowercases() {
        assert_eq!(sanitize_name("Hill Cigarette!"), "hill_cigarette");
    }

    #[test]
    fn caps_long_names_and_trims_edges() {
        let name = sanitize_name(&"a-".repeat(100));
        assert!(name.len() <= MAX_NAME_LEN, "len was {}", name.len());
        assert!(!name.starts_with('_') && !name.ends_with('_'), "got: {name}");
    }

    #[test]
    fn falls_back_when_empty() {
        assert_eq!(sanitize_name("!!!"), "emoji");
    }
}
