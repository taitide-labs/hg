use std::fmt;
use std::num::{NonZeroU16, NonZeroU32, NonZeroU64};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SeriesId(NonZeroU64);

impl SeriesId {
    #[must_use]
    pub const fn new(value: NonZeroU64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl TryFrom<u64> for SeriesId {
    type Error = &'static str;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or("series id must be non-zero")
    }
}

impl fmt::Display for SeriesId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EpisodeNumber(NonZeroU32);

impl EpisodeNumber {
    #[must_use]
    pub const fn new(value: NonZeroU32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

impl TryFrom<u32> for EpisodeNumber {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        NonZeroU32::new(value)
            .map(Self)
            .ok_or("episode number must be >= 1")
    }
}

impl fmt::Display for EpisodeNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommentId(NonZeroU64);

impl CommentId {
    #[must_use]
    pub const fn new(value: NonZeroU64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl TryFrom<u64> for CommentId {
    type Error = &'static str;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or("comment id must be non-zero")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EpisodeKey {
    pub series_id: SeriesId,
    pub ep_num: EpisodeNumber,
}

impl EpisodeKey {
    #[must_use]
    pub const fn new(series_id: SeriesId, ep_num: EpisodeNumber) -> Self {
        Self { series_id, ep_num }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRequest {
    pub page: NonZeroU32,
    pub size: NonZeroU16,
}

impl PageRequest {
    #[must_use]
    pub const fn new(page: NonZeroU32, size: NonZeroU16) -> Self {
        Self { page, size }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageInfo {
    pub page: u32,
    pub pages: u32,
    pub size: u32,
    pub total: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamProfile {
    Hevc,
    Main,
}
