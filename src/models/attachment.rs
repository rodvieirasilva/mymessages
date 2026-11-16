use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    Image,
    Audio,
    Video,
    Document,
    Sticker,
    Other,
}

impl MediaType {
    pub fn from_mimetype(mimetype: &str) -> Self {
        let lower = mimetype.to_lowercase();
        if lower.starts_with("image/webp") && lower.contains("sticker") {
            Self::Sticker
        } else if lower.starts_with("image/") {
            Self::Image
        } else if lower.starts_with("audio/") {
            Self::Audio
        } else if lower.starts_with("video/") {
            Self::Video
        } else if lower.starts_with("application/pdf")
            || lower.contains("officedocument")
            || lower.contains("msword")
            || lower.contains("text/")
        {
            Self::Document
        } else {
            Self::Other
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Document => "document",
            Self::Sticker => "sticker",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalAttachment {
    pub filename: Option<String>,
    pub mimetype: String,
    pub data: Vec<u8>,
    pub sha256: String,
    pub media_type: MediaType,
    pub duration_seconds: Option<u32>,
}

impl CanonicalAttachment {
    pub fn new(filename: Option<String>, mimetype: String, data: Vec<u8>) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let sha256 = hex::encode(hasher.finalize());
        let media_type = MediaType::from_mimetype(&mimetype);

        Self {
            filename,
            mimetype,
            data,
            sha256,
            media_type,
            duration_seconds: None,
        }
    }

    pub fn with_duration(mut self, seconds: u32) -> Self {
        self.duration_seconds = Some(seconds);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_type_detection() {
        assert_eq!(MediaType::from_mimetype("image/jpeg"), MediaType::Image);
        assert_eq!(MediaType::from_mimetype("audio/ogg; codecs=opus"), MediaType::Audio);
        assert_eq!(MediaType::from_mimetype("video/mp4"), MediaType::Video);
        assert_eq!(MediaType::from_mimetype("application/pdf"), MediaType::Document);
        assert_eq!(MediaType::from_mimetype("text/plain"), MediaType::Document);
        assert_eq!(MediaType::from_mimetype("application/octet-stream"), MediaType::Other);
        assert_eq!(MediaType::Image.as_str(), "image");
    }

    #[test]
    fn test_attachment_creation() {
        let data = b"hello image binary data".to_vec();
        let att = CanonicalAttachment::new(Some("photo.jpg".into()), "image/jpeg".into(), data.clone())
            .with_duration(12);

        assert_eq!(att.filename.as_deref(), Some("photo.jpg"));
        assert_eq!(att.media_type, MediaType::Image);
        assert_eq!(att.duration_seconds, Some(12));
        assert!(!att.sha256.is_empty());
    }
}
