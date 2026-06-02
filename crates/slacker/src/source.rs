//! Resolves a user-supplied input into a fetch plan plus a default emoji name.
//!
//! Supported inputs: GIPHY links, Tenor view pages, Imgur links, any direct
//! media URL (`.gif`, `.mp4`, `.webp`, `.webm`, `.mov`), local files, and `-`
//! for standard input.

use std::path::{Path, PathBuf};

use crate::error::Error;

/// A resolved input: where to read the media from and a default emoji name.
#[derive(Debug)]
pub struct Source {
    /// Default emoji name derived from the input when `--name` is absent.
    pub name_hint: String,
    /// How to obtain the source media.
    pub fetch: Fetch,
}

/// How `convert` should obtain the source media.
#[derive(Debug)]
pub enum Fetch {
    /// Download a direct media URL.
    Url(String),
    /// Fetch a Tenor view page, scrape its media URL, then download it.
    TenorPage(String),
    /// Read an existing local file in place.
    File(PathBuf),
    /// Read media bytes from standard input.
    Stdin,
}

const MEDIA_EXTENSIONS: [&str; 5] = ["gif", "mp4", "webp", "webm", "mov"];

/// Resolves a raw input string into a [`Source`].
///
/// # Errors
///
/// Returns an error when the input is an unrecognised URL, a missing local
/// file, or a GIPHY link without a plausible id.
pub fn resolve(input: &str) -> Result<Source, Error> {
    if input == "-" {
        return Ok(Source { name_hint: String::from("emoji"), fetch: Fetch::Stdin });
    }
    if input.contains("://") {
        return resolve_url(input);
    }
    resolve_local(input)
}

fn resolve_local(input: &str) -> Result<Source, Error> {
    let path = Path::new(input);
    if !path.is_file() {
        return Err(Error::unsupported_source(input.to_owned()));
    }
    let name_hint = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| String::from("emoji"));
    Ok(Source { name_hint, fetch: Fetch::File(path.to_path_buf()) })
}

fn resolve_url(input: &str) -> Result<Source, Error> {
    let clean = without_query(input);
    let Some(host) = host_of(clean) else {
        return Err(Error::unsupported_source(input.to_owned()));
    };

    if host_matches(host, "giphy.com") {
        return resolve_giphy(input, clean);
    }
    if host_matches(host, "tenor.com") {
        return Ok(resolve_tenor(input, clean, host));
    }
    if host_matches(host, "imgur.com") {
        return resolve_imgur(input, clean);
    }
    if has_media_extension(clean) {
        return Ok(Source { name_hint: stem_of(clean), fetch: Fetch::Url(input.to_owned()) });
    }
    Err(Error::unsupported_source(input.to_owned()))
}

fn resolve_giphy(input: &str, clean: &str) -> Result<Source, Error> {
    let Some(id) = giphy_id(clean) else {
        return Err(Error::bad_giphy_url(input.to_owned()));
    };
    let media_url = format!("https://media.giphy.com/media/{id}/giphy.gif");
    Ok(Source { name_hint: id.to_owned(), fetch: Fetch::Url(media_url) })
}

fn resolve_tenor(input: &str, clean: &str, host: &str) -> Source {
    if host.starts_with("media") || has_media_extension(clean) {
        return Source { name_hint: stem_of(clean), fetch: Fetch::Url(input.to_owned()) };
    }
    Source { name_hint: tenor_hint(clean), fetch: Fetch::TenorPage(input.to_owned()) }
}

fn resolve_imgur(input: &str, clean: &str) -> Result<Source, Error> {
    if has_media_extension(clean) {
        return Ok(Source { name_hint: stem_of(clean), fetch: Fetch::Url(input.to_owned()) });
    }
    // Albums and galleries are containers, not single images, and cannot map to
    // a direct `i.imgur.com/{id}` media URL.
    if clean.contains("/a/") || clean.contains("/gallery/") {
        return Err(Error::unsupported_source(input.to_owned()));
    }
    let Some(segment) = last_path_segment(clean) else {
        return Err(Error::unsupported_source(input.to_owned()));
    };
    let id = segment.split('.').next().unwrap_or(segment);
    if !is_imgur_id(id) {
        return Err(Error::unsupported_source(input.to_owned()));
    }
    let media_url = format!("https://i.imgur.com/{id}.gif");
    Ok(Source { name_hint: id.to_owned(), fetch: Fetch::Url(media_url) })
}

fn is_imgur_id(id: &str) -> bool {
    (4..=12).contains(&id.len()) && id.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

/// Scrapes a Tenor media URL from a fetched view-page body.
///
/// Prefers an animated `.gif`, falling back to an `.mp4` clip.
pub fn extract_tenor_media(html: &str) -> Option<String> {
    let mut gif = None;
    let mut mp4 = None;

    for part in html.split("https://") {
        let body: String = part.chars().take_while(|character| is_url_char(*character)).collect();
        if body.is_empty() {
            continue;
        }
        let url = format!("https://{body}");
        if !host_of(&url).is_some_and(|host| host_matches(host, "tenor.com")) {
            continue;
        }
        if gif.is_none() && url.ends_with(".gif") {
            gif = Some(url);
        } else if mp4.is_none() && url.ends_with(".mp4") {
            mp4 = Some(url);
        }
    }

    gif.or(mp4)
}

fn is_url_char(character: char) -> bool {
    !character.is_whitespace()
        && !matches!(character, '"' | '\'' | '<' | '>' | '(' | ')' | '\\' | '`')
}

fn giphy_id(clean: &str) -> Option<&str> {
    let segment = giphy_segment(clean)?;
    let raw = segment.rsplit('-').find(|part| !part.is_empty())?;
    if raw.len() >= 6 && raw.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        Some(raw)
    } else {
        None
    }
}

fn giphy_segment(clean: &str) -> Option<&str> {
    let mut last = None;
    let mut previous = None;
    for segment in clean.split('/') {
        if !segment.is_empty() {
            previous = last;
            last = Some(segment);
        }
    }
    match last {
        Some(value) if value.ends_with(".gif") => previous,
        value => value,
    }
}

fn tenor_hint(clean: &str) -> String {
    let Some(segment) = last_path_segment(clean) else {
        return String::from("emoji");
    };
    let mut tokens: Vec<&str> = segment.split('-').collect();
    while let Some(last) = tokens.last() {
        let drop =
            last.is_empty() || *last == "gif" || last.bytes().all(|byte| byte.is_ascii_digit());
        if drop && tokens.len() > 1 {
            tokens.pop();
        } else {
            break;
        }
    }
    let hint = tokens.join("-");
    if hint.is_empty() { segment.to_owned() } else { hint }
}

fn without_query(url: &str) -> &str {
    url.split(['?', '#']).next().unwrap_or(url)
}

fn host_of(url: &str) -> Option<&str> {
    let after = url.split("://").nth(1)?;
    let authority = after.split('/').next()?;
    let host = authority.rsplit('@').next().unwrap_or(authority);
    host.split(':').next()
}

fn host_matches(host: &str, base: &str) -> bool {
    host == base || host.ends_with(&format!(".{base}"))
}

fn last_path_segment(clean: &str) -> Option<&str> {
    clean.rsplit('/').find(|segment| !segment.is_empty())
}

fn extension_of(clean: &str) -> Option<String> {
    let segment = last_path_segment(clean)?;
    let ext = segment.rsplit('.').next()?;
    if ext == segment { None } else { Some(ext.to_ascii_lowercase()) }
}

fn has_media_extension(clean: &str) -> bool {
    extension_of(clean).is_some_and(|ext| MEDIA_EXTENSIONS.contains(&ext.as_str()))
}

fn stem_of(clean: &str) -> String {
    let segment = last_path_segment(clean).unwrap_or("emoji");
    let stem = segment.split('.').next().unwrap_or(segment);
    if stem.is_empty() { String::from("emoji") } else { stem.to_owned() }
}

#[cfg(test)]
mod tests {
    use super::{Fetch, extract_tenor_media, resolve};

    fn fetch_of(input: &str) -> Fetch {
        match resolve(input) {
            Ok(source) => source.fetch,
            Err(error) => panic!("resolve failed: {error}"),
        }
    }

    #[test]
    fn giphy_slug_builds_media_url() {
        match fetch_of("https://giphy.com/gifs/name-HB4aJElNd7JMas9WSU") {
            Fetch::Url(url) => {
                assert_eq!(url, "https://media.giphy.com/media/HB4aJElNd7JMas9WSU/giphy.gif");
            }
            other => panic!("unexpected fetch: {other:?}"),
        }
    }

    #[test]
    fn giphy_media_url_round_trips() {
        match fetch_of("https://media.giphy.com/media/HB4aJElNd7JMas9WSU/giphy.gif") {
            Fetch::Url(url) => {
                assert_eq!(url, "https://media.giphy.com/media/HB4aJElNd7JMas9WSU/giphy.gif");
            }
            other => panic!("unexpected fetch: {other:?}"),
        }
    }

    #[test]
    fn imgur_page_maps_to_direct_media() {
        match fetch_of("https://imgur.com/abc123") {
            Fetch::Url(url) => assert_eq!(url, "https://i.imgur.com/abc123.gif"),
            other => panic!("unexpected fetch: {other:?}"),
        }
    }

    #[test]
    fn direct_media_url_downloads_as_is() {
        match fetch_of("https://example.com/path/clip.mp4?token=1") {
            Fetch::Url(url) => assert_eq!(url, "https://example.com/path/clip.mp4?token=1"),
            other => panic!("unexpected fetch: {other:?}"),
        }
    }

    #[test]
    fn tenor_view_page_needs_scrape() {
        let source = match resolve("https://tenor.com/view/happy-cat-dancing-gif-12345678") {
            Ok(value) => value,
            Err(error) => panic!("resolve failed: {error}"),
        };
        assert_eq!(source.name_hint, "happy-cat-dancing");
        match source.fetch {
            Fetch::TenorPage(page) => {
                assert_eq!(page, "https://tenor.com/view/happy-cat-dancing-gif-12345678");
            }
            other => panic!("unexpected fetch: {other:?}"),
        }
    }

    #[test]
    fn stdin_marker_resolves_to_stdin() {
        assert!(matches!(fetch_of("-"), Fetch::Stdin));
    }

    #[test]
    fn unknown_url_is_rejected() {
        assert!(resolve("https://example.com/not-media").is_err());
    }

    #[test]
    fn imgur_album_is_rejected() {
        assert!(resolve("https://imgur.com/a/abc123").is_err());
        assert!(resolve("https://imgur.com/gallery/abc123").is_err());
    }

    #[test]
    fn tenor_scrape_prefers_gif_over_mp4() {
        let html = r#"<meta content="https://media1.tenor.com/m/key/clip.mp4">
            <meta property="og:image" content="https://media1.tenor.com/m/key/clip.gif">"#;
        assert_eq!(
            extract_tenor_media(html).as_deref(),
            Some("https://media1.tenor.com/m/key/clip.gif"),
        );
    }

    #[test]
    fn tenor_scrape_falls_back_to_mp4() {
        let html = r#"<source src="https://media.tenor.com/key/clip.mp4" type="video/mp4">"#;
        assert_eq!(
            extract_tenor_media(html).as_deref(),
            Some("https://media.tenor.com/key/clip.mp4"),
        );
    }
}
