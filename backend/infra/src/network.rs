use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkKind {
    Unmetered,
    Metered,
}

pub trait NetworkMonitor: Send + Sync {
    fn current_network(&self) -> NetworkKind;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemNetworkMonitor;

impl NetworkMonitor for SystemNetworkMonitor {
    fn current_network(&self) -> NetworkKind {
        NetworkKind::Unmetered
    }
}

#[derive(Debug)]
pub struct FixedNetworkMonitor {
    kind: AtomicU8,
}

impl FixedNetworkMonitor {
    #[must_use]
    pub const fn new(kind: NetworkKind) -> Self {
        Self {
            kind: AtomicU8::new(Self::encode(kind)),
        }
    }

    pub fn set(&self, kind: NetworkKind) {
        self.kind.store(Self::encode(kind), Ordering::SeqCst);
    }

    const fn encode(kind: NetworkKind) -> u8 {
        match kind {
            NetworkKind::Unmetered => 0,
            NetworkKind::Metered => 1,
        }
    }
}

impl NetworkMonitor for FixedNetworkMonitor {
    fn current_network(&self) -> NetworkKind {
        match self.kind.load(Ordering::SeqCst) {
            1 => NetworkKind::Metered,
            _ => NetworkKind::Unmetered,
        }
    }
}
