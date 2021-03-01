pub mod pane;
pub mod popup;
pub mod status;
pub mod tab;
pub mod window;

pub use pane::Pane;
pub use popup::Popup;
pub use status::Status;
pub use tab::Tab;
pub use window::Window;

#[derive(Copy, Clone, PartialEq)]
pub enum Type {
    NONE,
    LOG,
    POD,
    NS,
}
