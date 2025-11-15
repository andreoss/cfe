#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    Webp,
}

impl ImageFormat {
    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "image/png" => Some(Self::Png),
            "image/jpeg" => Some(Self::Jpeg),
            "image/gif" => Some(Self::Gif),
            "image/webp" => Some(Self::Webp),
            _ => None,
        }
    }

    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
            return Some(Self::Png);
        }
        if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            return Some(Self::Jpeg);
        }
        if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            return Some(Self::Gif);
        }
        if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
            return Some(Self::Webp);
        }
        None
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AvatarError {
    Empty,
    TooLarge,
    UnsupportedFormat,
}

pub const MAX_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Avatar {
    bytes: Vec<u8>,
    format: ImageFormat,
}

impl Avatar {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, AvatarError> {
        if bytes.is_empty() {
            return Err(AvatarError::Empty);
        }
        if bytes.len() > MAX_BYTES {
            return Err(AvatarError::TooLarge);
        }
        let format = ImageFormat::sniff(&bytes).ok_or(AvatarError::UnsupportedFormat)?;
        Ok(Self { bytes, format })
    }

    pub fn from_parts(bytes: Vec<u8>, format: ImageFormat) -> Self {
        Self { bytes, format }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn format(&self) -> ImageFormat {
        self.format
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png() -> Vec<u8> {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        bytes.extend_from_slice(b"body");
        bytes
    }

    #[test]
    fn accepts_each_supported_format() {
        let cases: Vec<(Vec<u8>, ImageFormat)> = vec![
            (png(), ImageFormat::Png),
            (vec![0xff, 0xd8, 0xff, 0x00], ImageFormat::Jpeg),
            (b"GIF89a....".to_vec(), ImageFormat::Gif),
            (b"RIFF____WEBPVP8 ".to_vec(), ImageFormat::Webp),
        ];
        for (bytes, expected) in cases {
            let avatar = Avatar::parse(bytes).unwrap();
            assert_eq!(avatar.format(), expected);
            assert_eq!(avatar.format().content_type(), expected.content_type());
        }
    }

    #[test]
    fn rejects_empty_bytes() {
        assert_eq!(Avatar::parse(Vec::new()), Err(AvatarError::Empty));
    }

    #[test]
    fn rejects_anything_over_the_cap() {
        let mut bytes = png();
        bytes.resize(MAX_BYTES + 1, 0);
        assert_eq!(Avatar::parse(bytes), Err(AvatarError::TooLarge));
    }

    #[test]
    fn accepts_exactly_the_cap() {
        let mut bytes = png();
        bytes.resize(MAX_BYTES, 0);
        assert!(Avatar::parse(bytes).is_ok());
    }

    #[test]
    fn refuses_a_document_pretending_to_be_an_image() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>";
        assert_eq!(
            Avatar::parse(svg.to_vec()),
            Err(AvatarError::UnsupportedFormat)
        );
        let html = b"<!doctype html><script>alert(1)</script>";
        assert_eq!(
            Avatar::parse(html.to_vec()),
            Err(AvatarError::UnsupportedFormat)
        );
    }

    #[test]
    fn the_format_comes_from_the_bytes_not_a_declared_type() {
        let avatar = Avatar::parse(png()).unwrap();
        assert_eq!(avatar.format(), ImageFormat::Png);
        assert_eq!(ImageFormat::parse("image/svg+xml"), None);
        assert_eq!(ImageFormat::parse("text/html"), None);
        assert_eq!(ImageFormat::parse("image/png"), Some(ImageFormat::Png));
    }

    #[test]
    fn a_truncated_webp_header_is_not_a_webp() {
        assert_eq!(ImageFormat::sniff(b"RIFF____"), None);
        assert_eq!(ImageFormat::sniff(b"RIFF____NOPE"), None);
    }

    #[test]
    fn keeps_the_bytes_it_was_given() {
        let avatar = Avatar::parse(png()).unwrap();
        assert_eq!(avatar.bytes(), png().as_slice());
    }
}
