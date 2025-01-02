#[cfg_attr(target_os = "linux", path = "clipboard/linux.rs")]
#[cfg_attr(not(target_os = "linux"), path = "clipboard/generic.rs")]
mod platform;

pub use platform::Clipboard;
