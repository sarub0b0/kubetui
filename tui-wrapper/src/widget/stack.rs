use std::{cmp::Ordering, collections::HashMap};

use super::{EventResult, RenderTrait, Widget, WidgetItem, WidgetTrait};

use crossterm::event::{KeyEvent, MouseEvent};
use event::UserEvent;

use crate::event::InnerCallback;

use tui::{backend::Backend, layout::Rect, Frame};

use derivative::*;

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct Stack<'a> {
    id: String,
    title: String,
    chunk: Rect,
    widgets: Vec<Widget<'a>>,
    current_index: usize,
    #[derivative(Debug = "ignore")]
    callbacks: HashMap<UserEvent, InnerCallback>,
}

impl<'a> Stack<'a> {
    fn push_widget(&mut self, w: Widget<'a>) {
        self.widgets.push(w);
    }

    fn current_widget(&self) -> &Widget<'a> {
        &self.widgets[self.current_index]
    }

    fn current_widget_mut(&mut self) -> &mut Widget<'a> {
        &mut self.widgets[self.current_index]
    }

    fn current_widget_chunk(&self) -> Rect {
        self.widgets[self.current_index].chunk()
    }
}

#[derive(Debug, Default)]
pub struct StackBuilder {
    id: String,
    title: String,
}

impl StackBuilder {
    fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
}

impl WidgetTrait for Stack<'_> {
    fn id(&self) -> &str {
        &self.id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn focusable(&self) -> bool {
        true
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn widget_item(&self) -> Option<WidgetItem> {
        self.current_widget().widget_item()
    }

    fn select_index(&mut self, _: usize) {
        todo!()
    }

    fn select_next(&mut self, _: usize) {
        todo!()
    }

    fn select_prev(&mut self, _: usize) {
        todo!()
    }

    fn select_first(&mut self) {
        todo!()
    }

    fn select_last(&mut self) {
        todo!()
    }

    fn append_widget_item(&mut self, _: WidgetItem) {
        todo!()
    }

    fn update_widget_item(&mut self, _: WidgetItem) {
        todo!()
    }

    fn update_chunk(&mut self, chunk: Rect) {
        debug_assert!(!self.widgets.is_empty());

        let center_index = if self.widgets.len() % 2 == 0 {
            (self.widgets.len() / 2).saturating_sub(1)
        } else {
            self.widgets.len() / 2
        };

        for (i, w) in self.widgets.iter_mut().enumerate() {
            let delta = (center_index as i16 - i as i16).unsigned_abs();

            let offset_x: u16 = 2 * delta;
            let offset_y: u16 = delta;

            let ch = match i.cmp(&center_index) {
                Ordering::Less => Rect::new(
                    chunk.x.saturating_sub(offset_x),
                    chunk.x.saturating_sub(offset_y),
                    chunk.width,
                    chunk.height,
                ),
                Ordering::Equal => chunk,
                Ordering::Greater => Rect::new(
                    chunk.x + offset_x,
                    chunk.y + offset_y,
                    chunk.width,
                    chunk.height,
                ),
            };

            w.update_chunk(ch);
        }
    }

    fn clear(&mut self) {
        todo!()
    }

    fn on_mouse_event(&mut self, _: MouseEvent) -> EventResult {
        todo!()
    }

    fn on_key_event(&mut self, _: KeyEvent) -> EventResult {
        todo!()
    }
}

impl RenderTrait for Stack<'_> {
    fn render<B>(&mut self, f: &mut Frame<'_, B>, _: bool)
    where
        B: Backend,
    {
        self.widgets[self.current_index].render(f, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod update_chunk {
        use crate::widget::Text;
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn chunk_position_one_widget() {
            // "  ┌Title─┐  ",
            // "  │      │  ",
            // "  │      │  ",
            // "  │      │  ",
            // "  │      │  ",
            // "  │      │  ",
            // "  │      │  ",
            // "  └──────┘  ",
            let base_chunk = Rect::new(50, 50, 50, 50);

            let mut w = Stack::default();

            w.push_widget(Widget::Text(Text::default()));

            w.update_chunk(base_chunk);

            let expected = Rect::new(50, 50, 50, 50);

            assert_eq!(expected, w.widgets[0].chunk());
        }

        #[test]
        fn chunk_position_even_widgets() {
            // "  ┌Title─┐        ",
            // "  │ ┌Title─┐      ",
            // "  │ │ ┌Title─┐    ",
            // "  │ │ │ ┌Title─┐  ",
            // "  │ │ │ │      │  ",
            // "  │ │ │ │      │  ",
            // "  │ │ │ │      │  ",
            // "  └ │ │ │      │  ",
            // "    └ │ │      │  ",
            // "      └ │      │  ",
            // "        └──────┘  ",

            let base_chunk = Rect::new(50, 50, 50, 50);

            let mut w = Stack::default();

            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));

            w.update_chunk(base_chunk);

            assert_eq!(Rect::new(48, 49, 50, 50), w.widgets[0].chunk());
            assert_eq!(Rect::new(50, 50, 50, 50), w.widgets[1].chunk());
            assert_eq!(Rect::new(52, 51, 50, 50), w.widgets[2].chunk());
            assert_eq!(Rect::new(54, 52, 50, 50), w.widgets[3].chunk());
        }

        #[test]
        fn chunk_position_odd_widgets() {
            // "  ┌Title─┐      ",
            // "  │ ┌Title─┐    ",
            // "  │ │ ┌Title─┐  ",
            // "  │ │ │      │  ",
            // "  │ │ │      │  ",
            // "  │ │ │      │  ",
            // "  │ │ │      │  ",
            // "  └ │ │      │  ",
            // "    └ │      │  ",
            // "      └──────┘  ",

            let base_chunk = Rect::new(50, 50, 50, 50);

            let mut w = Stack::default();

            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));

            w.update_chunk(base_chunk);

            assert_eq!(Rect::new(46, 48, 50, 50), w.widgets[0].chunk());
            assert_eq!(Rect::new(48, 49, 50, 50), w.widgets[1].chunk());
            assert_eq!(Rect::new(50, 50, 50, 50), w.widgets[2].chunk());
            assert_eq!(Rect::new(52, 51, 50, 50), w.widgets[3].chunk());
            assert_eq!(Rect::new(54, 52, 50, 50), w.widgets[4].chunk());
        }
    }
    mod e2e {
        use super::*;
        use tui::{backend::TestBackend, buffer::Buffer, Terminal};

        // #[test]
        // fn sample() {
        //     let backend = TestBackend::new(20, 10);
        //     let mut terminal = Terminal::new(backend).unwrap();

        //     terminal.draw(|f| {}).unwrap();

        //     let expected = Buffer::with_lines::<&'static str>(vec![
        //         "                    ",
        //         "                    ",
        //         "                    ",
        //         "                    ",
        //         "                    ",
        //         "                    ",
        //         "                    ",
        //         "                    ",
        //         "                    ",
        //         "                    ",
        //     ]);

        //     terminal.backend().assert_buffer(&expected)
        // }
    }
}
