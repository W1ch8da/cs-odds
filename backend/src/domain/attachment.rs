use uuid::Uuid;

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AttachmentId(pub Uuid);

pub const MAX_FILE_BYTES: i64 = 25 * 1024 * 1024;
pub const MAX_FILES_PER_MESSAGE: usize = 10;

/// A file name that is safe to store and to send back in a
/// `Content-Disposition` header: no path, no control characters, at most
/// 200 characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileName(String);

impl FileName {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        // Browsers on some systems send full paths; keep only the last part.
        let base = raw.rsplit(['/', '\\']).next().unwrap_or_default();
        let cleaned: String = base.chars().filter(|c| !c.is_control() && *c != '"').collect();
        let cleaned = cleaned.trim().trim_start_matches('.').trim();
        if cleaned.is_empty() {
            return Err(DomainError::InvalidFile("This file has no usable name. Rename it and try again."));
        }
        // Keep the extension when shortening long names.
        let name = if cleaned.chars().count() > 200 {
            let (stem, ext) = cleaned.rsplit_once('.').filter(|(_, e)| e.len() <= 10).unwrap_or((cleaned, ""));
            let keep = 200 - ext.chars().count() - 1;
            let stem: String = stem.chars().take(keep).collect();
            if ext.is_empty() { stem } else { format!("{stem}.{ext}") }
        } else {
            cleaned.to_owned()
        };
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A `type/subtype` media type, lower-cased, without parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentType(String);

impl ContentType {
    pub const FALLBACK: &'static str = "application/octet-stream";

    /// Unknown or malformed types become `application/octet-stream`, since
    /// browsers often send nothing for uncommon files.
    pub fn parse_or_default(raw: &str) -> Self {
        let essence = raw.split(';').next().unwrap_or_default().trim().to_ascii_lowercase();
        let valid = essence.split_once('/').is_some_and(|(t, s)| {
            let ok = |p: &str| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || "!#$&-^_.+".contains(c));
            ok(t) && ok(s)
        });
        Self(if valid && essence.len() <= 100 { essence } else { Self::FALLBACK.to_owned() })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn validate_size(size: i64) -> Result<(), DomainError> {
    if size <= 0 {
        return Err(DomainError::InvalidFile("This file is empty."));
    }
    if size > MAX_FILE_BYTES {
        return Err(DomainError::InvalidFile("Files can be up to 25 MB. Try a smaller file or a share link."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_are_cleaned() {
        assert_eq!(FileName::parse("C:\\Users\\jordan\\bank statement.pdf").unwrap().as_str(), "bank statement.pdf");
        assert_eq!(FileName::parse("../../etc/passwd").unwrap().as_str(), "passwd");
        assert_eq!(FileName::parse("  report\u{0}\n\"final\".csv ").unwrap().as_str(), "reportfinal.csv");
        assert_eq!(FileName::parse(".env").unwrap().as_str(), "env");
        assert!(FileName::parse("   ").is_err());
        assert!(FileName::parse("folder/").is_err());
    }

    #[test]
    fn long_names_keep_their_extension() {
        let name = FileName::parse(&format!("{}.xlsx", "a".repeat(300))).unwrap();
        assert_eq!(name.as_str().chars().count(), 200);
        assert!(name.as_str().ends_with(".xlsx"));
    }

    #[test]
    fn content_types() {
        assert_eq!(ContentType::parse_or_default("Image/PNG").as_str(), "image/png");
        assert_eq!(ContentType::parse_or_default("text/plain; charset=utf-8").as_str(), "text/plain");
        for bad in ["", "png", "image/", "a b/c", "x/y\r\nSet-Cookie: 1"] {
            assert_eq!(ContentType::parse_or_default(bad).as_str(), ContentType::FALLBACK, "{bad:?}");
        }
    }

    #[test]
    fn sizes() {
        assert!(validate_size(0).is_err());
        assert!(validate_size(MAX_FILE_BYTES).is_ok());
        assert!(validate_size(MAX_FILE_BYTES + 1).is_err());
    }
}
