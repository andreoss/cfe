use crate::{ImageFormat, TopicId, UserId};
use time::OffsetDateTime;

pub const MAX_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_PER_TOPIC: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AttachmentId(uuid::Uuid);

impl AttachmentId {
    pub fn new(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AttachmentError {
    Empty,
    TooLarge,
    UnsupportedFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    id: AttachmentId,
    topic_id: TopicId,
    format: ImageFormat,
    bytes: Vec<u8>,
    uploaded_by: UserId,
    uploaded_at: OffsetDateTime,
}

impl Attachment {
    pub fn parse(
        id: AttachmentId,
        topic_id: TopicId,
        bytes: Vec<u8>,
        uploaded_by: UserId,
        uploaded_at: OffsetDateTime,
    ) -> Result<Self, AttachmentError> {
        if bytes.is_empty() {
            return Err(AttachmentError::Empty);
        }
        if bytes.len() > MAX_BYTES {
            return Err(AttachmentError::TooLarge);
        }
        let format = ImageFormat::sniff(&bytes).ok_or(AttachmentError::UnsupportedFormat)?;
        Ok(Self {
            id,
            topic_id,
            format,
            bytes,
            uploaded_by,
            uploaded_at,
        })
    }

    pub fn from_parts(
        id: AttachmentId,
        topic_id: TopicId,
        format: ImageFormat,
        bytes: Vec<u8>,
        uploaded_by: UserId,
        uploaded_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            topic_id,
            format,
            bytes,
            uploaded_by,
            uploaded_at,
        }
    }

    pub fn id(&self) -> AttachmentId {
        self.id
    }

    pub fn topic_id(&self) -> TopicId {
        self.topic_id
    }

    pub fn format(&self) -> ImageFormat {
        self.format
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn uploaded_by(&self) -> UserId {
        self.uploaded_by
    }

    pub fn uploaded_at(&self) -> OffsetDateTime {
        self.uploaded_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

    fn id() -> AttachmentId {
        AttachmentId::new(uuid::Uuid::from_u128(1))
    }

    fn topic() -> TopicId {
        TopicId::new(uuid::Uuid::from_u128(2))
    }

    fn who() -> UserId {
        UserId::new(uuid::Uuid::from_u128(3))
    }

    fn parse(bytes: Vec<u8>) -> Result<Attachment, AttachmentError> {
        Attachment::parse(id(), topic(), bytes, who(), OffsetDateTime::UNIX_EPOCH)
    }

    #[test]
    fn an_image_is_accepted_and_knows_what_it_is() {
        let attached = parse(PNG.to_vec()).unwrap();
        assert_eq!(attached.format(), ImageFormat::Png);
        assert_eq!(attached.format().content_type(), "image/png");
        assert_eq!(attached.topic_id(), topic());
        assert_eq!(attached.uploaded_by(), who());
    }

    #[test]
    fn every_format_the_application_serves_is_accepted() {
        assert_eq!(
            parse(vec![0xff, 0xd8, 0xff, 0x00]).unwrap().format(),
            ImageFormat::Jpeg
        );
        assert_eq!(
            parse(b"GIF89a...".to_vec()).unwrap().format(),
            ImageFormat::Gif
        );
        let mut webp = b"RIFF".to_vec();
        webp.extend_from_slice(&[0, 0, 0, 0]);
        webp.extend_from_slice(b"WEBP");
        assert_eq!(parse(webp).unwrap().format(), ImageFormat::Webp);
    }

    #[test]
    fn nothing_at_all_is_refused() {
        assert_eq!(parse(Vec::new()), Err(AttachmentError::Empty));
    }

    #[test]
    fn something_larger_than_allowed_is_refused() {
        let mut big = PNG.to_vec();
        big.resize(MAX_BYTES + 1, 0);
        assert_eq!(parse(big), Err(AttachmentError::TooLarge));
    }

    #[test]
    fn one_exactly_at_the_limit_is_accepted() {
        let mut edge = PNG.to_vec();
        edge.resize(MAX_BYTES, 0);
        assert!(parse(edge).is_ok());
    }

    #[test]
    fn something_that_is_not_an_image_is_refused_whatever_it_claims() {
        assert_eq!(
            parse(b"<script>alert(1)</script>".to_vec()),
            Err(AttachmentError::UnsupportedFormat)
        );
        assert_eq!(
            parse(b"%PDF-1.4".to_vec()),
            Err(AttachmentError::UnsupportedFormat)
        );
        assert_eq!(
            parse(b"<svg onload=alert(1)>".to_vec()),
            Err(AttachmentError::UnsupportedFormat)
        );
    }

    #[test]
    fn what_it_is_is_read_from_the_bytes_not_from_what_it_says() {
        let mut lying = PNG.to_vec();
        lying.extend_from_slice(b"but claims to be a document");
        assert_eq!(parse(lying).unwrap().format(), ImageFormat::Png);
    }

    #[test]
    fn a_stored_one_is_rebuilt_from_its_parts() {
        let stored = Attachment::from_parts(
            id(),
            topic(),
            ImageFormat::Png,
            PNG.to_vec(),
            who(),
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(stored.id(), id());
        assert_eq!(stored.bytes(), &PNG);
        assert_eq!(stored.uploaded_at(), OffsetDateTime::UNIX_EPOCH);
    }
}
