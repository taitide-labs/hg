pub mod commands;
pub mod layout;
pub mod pool;
pub mod state;

#[cfg(target_os = "windows")]
mod component;
#[cfg(target_os = "windows")]
mod mpv;

#[cfg(target_os = "windows")]
pub use component::register;

#[cfg(test)]
mod tests;
