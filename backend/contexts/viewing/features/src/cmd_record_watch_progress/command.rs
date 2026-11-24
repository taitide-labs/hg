use hg_contracts::error::ContractError;
use hg_contracts::ids::EpisodeKey;
use hg_contracts::viewing::ReportWatchProgressRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordWatchProgressCommand {
    pub key: EpisodeKey,
    pub position_secs: u32,
    pub duration_secs: u32,
}

impl TryFrom<ReportWatchProgressRequest> for RecordWatchProgressCommand {
    type Error = ContractError;

    fn try_from(request: ReportWatchProgressRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            key: request.key,
            position_secs: request.position_secs,
            duration_secs: request.duration_secs,
        })
    }
}
