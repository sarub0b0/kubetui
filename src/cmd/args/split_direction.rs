use ratatui::layout::Direction;
use strum::EnumString;

#[derive(Debug, Default, EnumString, Clone, Copy, PartialEq, Eq)]
#[strum(ascii_case_insensitive)]
pub enum SplitDirection {
    #[strum(serialize = "horizontal", serialize = "h")]
    Horizontal,
    #[default]
    #[strum(serialize = "vertical", serialize = "v")]
    Vertical,
}

impl From<SplitDirection> for Direction {
    fn from(value: SplitDirection) -> Self {
        match value {
            SplitDirection::Vertical => Direction::Vertical,
            SplitDirection::Horizontal => Direction::Horizontal,
        }
    }
}

impl SplitDirection {
    pub fn to_direction(self) -> Direction {
        match self {
            SplitDirection::Vertical => Direction::Vertical,
            SplitDirection::Horizontal => Direction::Horizontal,
        }
    }
}
