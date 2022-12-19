use crate::errors;
use std::path::PathBuf;
use std::str::FromStr;

/// Enum covering all supported renderers
///
/// Typesafety first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedRenderer {
    Tectonic,
    Latex,
    Markdown,
    Html,
}

impl FromStr for SupportedRenderer {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "tectonic" => Self::Tectonic,
            "latex" => Self::Latex,
            "markdown" => Self::Markdown,
            "html" => Self::Html,
            s => return Err(errors::Error::RendererNotSupported(s.to_owned())),
        })
    }
}

/// A dollar sign or maybe two, or three.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dollar<'a> {
    Start(&'a str),
    End(&'a str),
    Empty,
}

impl Dollar<'_> {
    pub fn is_block(&self) -> bool {
        self.as_ref().starts_with("$$")
    }
}

impl<'a> AsRef<str> for Dollar<'a> {
    fn as_ref(&self) -> &'a str {
        match self {
            Self::Start(s) => s,
            Self::End(s) => s,
            Self::Empty => "",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LiCo {
    /// Base 1 line number
    pub lineno: usize,
    /// Base 1 column number
    pub column: usize,
}

/// A content reference
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content<'a> {
    /// Content between `start` and `end` including.
    pub s: &'a str,
    /// From (including!)
    pub start: LiCo,
    /// Until (including!)
    pub end: LiCo,
    /// Byte range that can be used with the original to extract `s`
    pub byte_range: std::ops::Range<usize>,
    /// Enclosing delimiter
    pub delimiter: Dollar<'a>,
}

impl<'a> AsRef<str> for Content<'a> {
    fn as_ref(&self) -> &str {
        self.s
    }
}

impl<'a> std::ops::Deref for Content<'a> {
    type Target = &'a str;
    fn deref(&self) -> &Self::Target {
        &self.s
    }
}

/// Parsed content reference with a path to the replacement svg
pub struct Replacement<'a> {
    pub content: Content<'a>,

    /// Intermediate representation if there is any, directly usable with latex/tectonic backends;.
    pub(crate) intermediate: Option<String>,
    pub svg: PathBuf,
}

impl<'a> Replacement<'a> {
    pub fn intermediate(&self) -> &str {
        if let Some(ref intermediate) = self.intermediate {
            intermediate.as_str()
        } else {
            self.content.s
        }
    }
}
