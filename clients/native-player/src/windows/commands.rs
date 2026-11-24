use crate::protocol::{PropChange, VideoPlayerProps};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MpvOption {
    pub name: &'static str,
    pub value: &'static str,
}

pub const DEFAULT_INIT_OPTIONS: &[MpvOption] = &[
    MpvOption {
        name: "vo",
        value: "libmpv",
    },
    MpvOption {
        name: "hwdec",
        value: "d3d11va-copy",
    },
    MpvOption {
        name: "keep-open",
        value: "yes",
    },
    MpvOption {
        name: "cache",
        value: "yes",
    },
    MpvOption {
        name: "osc",
        value: "no",
    },
    MpvOption {
        name: "osd-level",
        value: "0",
    },
];

#[derive(Debug, Clone, PartialEq)]
pub enum MpvCommand {
    LoadFile { url: String },
    Stop,
    SetPaused(bool),
    SetRate(f64),
    SetVolume(f64),
    SetMuted(bool),
    SeekAbsolute(u32),
}

#[must_use]
pub fn volume_to_mpv_percent(volume: f64) -> f64 {
    if !volume.is_finite() {
        return 80.0;
    }
    volume.clamp(0.0, 1.0) * 100.0
}

#[must_use]
pub fn commands_for_prop_change(props: &VideoPlayerProps, change: PropChange) -> Vec<MpvCommand> {
    match change {
        PropChange::Source => match &props.src {
            Some(url) => vec![
                MpvCommand::LoadFile { url: url.clone() },
                MpvCommand::SetRate(props.rate),
                MpvCommand::SetVolume(volume_to_mpv_percent(props.volume)),
                MpvCommand::SetMuted(props.muted),
                MpvCommand::SetPaused(props.paused),
            ],
            None => vec![MpvCommand::Stop],
        },
        PropChange::Paused => vec![MpvCommand::SetPaused(props.paused)],
        PropChange::Rate => vec![MpvCommand::SetRate(props.rate)],
        PropChange::Volume => vec![MpvCommand::SetVolume(volume_to_mpv_percent(props.volume))],
        PropChange::Muted => vec![MpvCommand::SetMuted(props.muted)],
        PropChange::Seek { position_secs } => vec![MpvCommand::SeekAbsolute(position_secs)],
        PropChange::Unchanged | PropChange::Aspect => Vec::new(),
    }
}

#[must_use]
pub fn commands_for_attach(props: &VideoPlayerProps) -> Vec<MpvCommand> {
    let mut commands = Vec::with_capacity(5);
    if let Some(url) = &props.src {
        commands.push(MpvCommand::LoadFile { url: url.clone() });
    }
    commands.push(MpvCommand::SetRate(props.rate));
    commands.push(MpvCommand::SetVolume(volume_to_mpv_percent(props.volume)));
    commands.push(MpvCommand::SetMuted(props.muted));
    commands.push(MpvCommand::SetPaused(props.paused));
    commands
}

#[must_use]
pub fn commands_for_recycle() -> Vec<MpvCommand> {
    vec![
        MpvCommand::Stop,
        MpvCommand::SetPaused(true),
        MpvCommand::SetRate(1.0),
        MpvCommand::SetVolume(80.0),
        MpvCommand::SetMuted(false),
    ]
}
