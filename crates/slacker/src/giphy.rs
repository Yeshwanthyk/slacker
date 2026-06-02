//! GIPHY URL parsing.

use crate::error::Error;

#[derive(Debug)]
/// Parsed GIPHY image reference.
pub struct Gif {
    pub(super) id: String,
    pub(super) media_url: String,
}

/// Extracts a GIPHY id and direct media URL from a GIPHY link.
///
/// # Errors
///
/// Returns an error when the URL does not contain a plausible GIPHY id.
pub fn parse(url: &str) -> Result<Gif, Error> {
    let clean = without_query(url);
    let Some(segment) = last_segment(clean) else {
        return Err(Error::bad_giphy_url(url.to_owned()));
    };
    let Some(raw_id) = id_part(segment) else {
        return Err(Error::bad_giphy_url(url.to_owned()));
    };
    if !valid_id(raw_id) {
        return Err(Error::bad_giphy_url(url.to_owned()));
    }

    let id = raw_id.to_owned();
    let media_url = format!("https://media.giphy.com/media/{id}/giphy.gif");
    Ok(Gif { id, media_url })
}

fn without_query(url: &str) -> &str {
    url.split(['?', '#']).next().map_or(url, |value| value)
}

fn last_segment(url: &str) -> Option<&str> {
    let mut last = None;
    let mut previous = None;

    for segment in url.split('/') {
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

fn id_part(segment: &str) -> Option<&str> {
    segment.rsplit('-').find(|part| !part.is_empty())
}

fn valid_id(id: &str) -> bool {
    id.len() >= 6 && id.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn extracts_id_from_giphy_slug() {
        let gif = match parse("https://giphy.com/gifs/name-HB4aJElNd7JMas9WSU") {
            Ok(value) => value,
            Err(error) => panic!("parse failed: {error}"),
        };

        assert_eq!(gif.id, "HB4aJElNd7JMas9WSU");
    }

    #[test]
    fn extracts_id_from_media_url() {
        let gif = match parse("https://media.giphy.com/media/HB4aJElNd7JMas9WSU/giphy.gif") {
            Ok(value) => value,
            Err(error) => panic!("parse failed: {error}"),
        };

        assert_eq!(gif.id, "HB4aJElNd7JMas9WSU");
    }
}
