pub mod pane;
pub mod tab;
pub mod window;

pub use pane::Pane;
pub use tab::Tab;
pub use window::Window;

#[derive(Copy, Clone, PartialEq)]
pub enum Type {
    NONE,
    LOG,
    POD,
}
