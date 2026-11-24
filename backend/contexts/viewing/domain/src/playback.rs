use hg_backend_contracts::catalog::PlaybackTicket;
use hg_backend_contracts::offline::CommittedOfflineAsset;
use hg_contracts::ids::StreamProfile;
use hg_contracts::viewing::{OnlineStreamKind, PlaybackOrigin};

use crate::error::ViewingError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedPlaybackPlan {
    OfflineLocal {
        playlist_rel_path: String,
        profile: StreamProfile,
        duration_secs: u32,
    },
    OnlineStream {
        stream_url: String,
        kind: OnlineStreamKind,
    },
}

impl ResolvedPlaybackPlan {
    #[must_use]
    pub const fn origin(&self) -> PlaybackOrigin {
        match self {
            Self::OfflineLocal { profile, .. } => {
                PlaybackOrigin::OfflineLocal { profile: *profile }
            }
            Self::OnlineStream { kind, .. } => PlaybackOrigin::OnlineStream { kind: *kind },
        }
    }
}

pub fn resolve_playback_plan(
    offline_asset: Option<&CommittedOfflineAsset>,
    online_ticket: Option<&PlaybackTicket>,
    preferred_profile: StreamProfile,
) -> Result<ResolvedPlaybackPlan, ViewingError> {
    if let Some(asset) = offline_asset {
        return Ok(ResolvedPlaybackPlan::OfflineLocal {
            playlist_rel_path: asset.playlist_rel_path.clone(),
            profile: asset.profile,
            duration_secs: asset.duration_secs,
        });
    }

    let ticket = online_ticket.ok_or(ViewingError::NoPlayableStream)?;
    let ordered = match preferred_profile {
        StreamProfile::Hevc => [
            (OnlineStreamKind::Hevc, ticket.streams.hevc.as_deref()),
            (OnlineStreamKind::Main, ticket.streams.main.as_deref()),
            (OnlineStreamKind::Preview, ticket.streams.preview.as_deref()),
        ],
        StreamProfile::Main => [
            (OnlineStreamKind::Main, ticket.streams.main.as_deref()),
            (OnlineStreamKind::Hevc, ticket.streams.hevc.as_deref()),
            (OnlineStreamKind::Preview, ticket.streams.preview.as_deref()),
        ],
    };

    for (kind, candidate) in ordered {
        if let Some(url) = candidate.map(str::trim).filter(|u| !u.is_empty()) {
            return Ok(ResolvedPlaybackPlan::OnlineStream {
                stream_url: url.to_owned(),
                kind,
            });
        }
    }

    Err(ViewingError::NoPlayableStream)
}
