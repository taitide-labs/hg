pub mod app;

pub use app::{
    AssembledContexts, BackendRuntime, BackendRuntimeConfig, REGISTERED_CONTEXTS, runtime_name,
};
pub use hg_catalog_features as catalog;
pub use hg_infra as infra;
pub use hg_offline_features as offline;
pub use hg_viewing_features as viewing;

#[cfg(test)]
mod tests;
