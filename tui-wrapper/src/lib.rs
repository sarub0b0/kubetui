pub mod view;
pub mod widget;

pub use view::*;
pub use widget::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
