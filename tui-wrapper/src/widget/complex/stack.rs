use std::{cmp::Ordering, collections::HashMap, rc::Rc};

use crossterm::event::{KeyEvent, MouseEvent};
use event::UserEvent;

use crate::{
    event::{EventResult, InnerCallback},
    widget::{config::WidgetConfig, Item, RenderTrait, Widget, WidgetTrait},
    Window,
};

use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};

use derivative::*;

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct Stack<'a> {
    id: String,
    widget_config: WidgetConfig,
    chunk: Rect,
    widgets: Vec<Widget<'a>>,
    current_index: usize,
    #[derivative(Debug = "ignore")]
    callbacks: HashMap<UserEvent, InnerCallback>,
}

impl<'a> Stack<'a> {
    pub fn builder() -> StackBuilder<'static> {
        StackBuilder::default()
    }

    pub fn push_widget(&mut self, w: impl Into<Widget<'a>>) {
        self.widgets.push(w.into());
    }

    pub fn current_widget(&self) -> &Widget<'a> {
        &self.widgets[self.current_index]
    }

    pub fn current_widget_mut(&mut self) -> &mut Widget<'a> {
        &mut self.widgets[self.current_index]
    }

    pub fn current_widget_chunk(&self) -> Rect {
        self.widgets[self.current_index].chunk()
    }

    pub fn next_widget(&mut self) {
        self.current_index += 1;
    }
}

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct StackBuilder<'a> {
    id: String,
    widget_config: WidgetConfig,
    widgets: Vec<Widget<'a>>,
    #[derivative(Debug = "ignore")]
    callbacks: HashMap<UserEvent, InnerCallback>,
}

impl<'a> StackBuilder<'a> {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn widget_config(mut self, widget_config: &WidgetConfig) -> Self {
        self.widget_config = widget_config.clone();
        self
    }

    pub fn widgets(mut self, widgets: impl Into<Vec<Widget<'a>>>) -> Self {
        self.widgets = widgets.into();
        self
    }

    pub fn widget(mut self, widget: Widget<'a>) -> Self {
        self.widgets.push(widget);
        self
    }

    pub fn action<F, E>(mut self, ev: E, cb: F) -> Self
    where
        E: Into<UserEvent>,
        F: Fn(&mut Window) -> EventResult + 'static,
    {
        self.callbacks.insert(ev.into(), Rc::new(cb));
        self
    }

    pub fn build(self) -> Stack<'a> {
        Stack {
            id: self.id,
            widget_config: self.widget_config,
            chunk: Default::default(),
            widgets: self.widgets,
            current_index: 0,
            callbacks: self.callbacks,
        }
    }
}

impl WidgetTrait for Stack<'_> {
    fn id(&self) -> &str {
        &self.id
    }

    fn widget_config(&self) -> &WidgetConfig {
        &self.widget_config
    }

    fn widget_config_mut(&mut self) -> &mut WidgetConfig {
        &mut self.widget_config
    }

    fn focusable(&self) -> bool {
        true
    }

    fn widget_item(&self) -> Option<Item> {
        self.current_widget().widget_item()
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn select_index(&mut self, i: usize) {
        self.current_widget_mut().select_index(i)
    }

    fn select_next(&mut self, i: usize) {
        self.current_widget_mut().select_next(i)
    }

    fn select_prev(&mut self, i: usize) {
        self.current_widget_mut().select_prev(i)
    }

    fn select_first(&mut self) {
        self.current_widget_mut().select_first()
    }

    fn select_last(&mut self) {
        self.current_widget_mut().select_last()
    }

    fn append_widget_item(&mut self, item: Item) {
        self.current_widget_mut().append_widget_item(item)
    }

    fn update_widget_item(&mut self, item: Item) {
        self.current_widget_mut().update_widget_item(item)
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        todo!()
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        // TODO 選択されたらindexをインクリメントしてwidgetをとじていく
        // TODO 洗濯されたアイテムをためていく
        // TODO 最後にコールバックを返す

        todo!()
    }

    fn update_chunk(&mut self, chunk: Rect) {
        // for w in self.widgets.iter_mut() {
        //     w.update_chunk(chunk);
        // }

        // let center_index = if self.widgets.len() % 2 == 0 {
        //     (self.widgets.len() / 2).saturating_sub(1)
        // } else {
        //     self.widgets.len() / 2
        // };
        let center_index = self.widgets.len() / 2;

        for (i, w) in self.widgets.iter_mut().enumerate() {
            let delta = (center_index as i16 - i as i16).unsigned_abs();

            let offset_x: u16 = 2 * delta;
            let offset_y: u16 = delta;

            let ch = match i.cmp(&center_index) {
                Ordering::Less => Rect::new(
                    chunk.x.saturating_sub(offset_x),
                    chunk.y + offset_y,
                    chunk.width,
                    chunk.height,
                ),
                Ordering::Equal => chunk,
                Ordering::Greater => Rect::new(
                    chunk.x + offset_x,
                    chunk.y.saturating_sub(offset_y),
                    chunk.width,
                    chunk.height,
                ),
            };

            w.update_chunk(ch);
        }
    }

    fn clear(&mut self) {
        self.widgets.iter_mut().for_each(|w| w.clear());
        self.current_index = 0;

        *(self.widget_config.append_title_mut()) = None;
    }
}

impl RenderTrait for Stack<'_> {
    fn render<B>(&mut self, f: &mut Frame<'_, B>, _selected: bool)
    where
        B: Backend,
    {
        if self.widgets.is_empty() {
            return;
        }

        for i in (self.current_index..self.widgets.len()).rev() {
            f.render_widget(Clear, self.widgets[i].chunk());
            self.widgets[i].render(f, self.current_index == i);
        }
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
            // "        ┌─ T3 ─┐ ",
            // "      ┌─ T2 ─┐ │ ", T2 = center
            // "    ┌─ T1 ─┐ │ │ ",
            // "  ┌─ T0 ─┐ │ │ │ ",
            // "  │      │ │ │ │ ",
            // "  │      │ │ │ │ ",
            // "  │      │ │ │ │ ",
            // "  │      │ │ │─┘ ",
            // "  │      │ │─┘   ",
            // "  │      │─┘     ",
            // "  └──────┘       ",

            let base_chunk = Rect::new(50, 50, 50, 50);

            let mut w = Stack::default();

            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));

            w.update_chunk(base_chunk);

            assert_eq!(Rect::new(54, 48, 50, 50), w.widgets[5].chunk());
            assert_eq!(Rect::new(52, 49, 50, 50), w.widgets[4].chunk());
            assert_eq!(Rect::new(50, 50, 50, 50), w.widgets[3].chunk()); // center
            assert_eq!(Rect::new(48, 51, 50, 50), w.widgets[2].chunk());
            assert_eq!(Rect::new(46, 52, 50, 50), w.widgets[1].chunk());
            assert_eq!(Rect::new(44, 53, 50, 50), w.widgets[0].chunk());
        }

        #[test]
        fn chunk_position_odd_widgets() {
            // "      ┌─ T2 ─┐   ",
            // "    ┌─ T1 ─┐ │   ", T1 = center
            // "  ┌─ T0 ─┐ │ │   ",
            // "  │      │ │ │   ",
            // "  │      │ │ │   ",
            // "  │      │ │ │   ",
            // "  │      │ │ │   ",
            // "  │      │ │─┘   ",
            // "  │      │─┘     ",
            // "  └──────┘       ",

            let base_chunk = Rect::new(50, 50, 50, 50);

            let mut w = Stack::default();

            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));
            w.push_widget(Widget::Text(Text::default()));

            w.update_chunk(base_chunk);

            assert_eq!(Rect::new(54, 48, 50, 50), w.widgets[4].chunk());
            assert_eq!(Rect::new(52, 49, 50, 50), w.widgets[3].chunk());
            assert_eq!(Rect::new(50, 50, 50, 50), w.widgets[2].chunk()); // center
            assert_eq!(Rect::new(48, 51, 50, 50), w.widgets[1].chunk());
            assert_eq!(Rect::new(46, 52, 50, 50), w.widgets[0].chunk());
        }
    }
    mod e2e {
        use super::*;

        use crate::{
            util::child_window_chunk,
            widget::{List, RenderTrait, WidgetTrait},
        };
        use tui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};

        #[test]
        fn chunk_position_odd_widgets() {
            let backend = TestBackend::new(46, 20);
            let mut terminal = Terminal::new(backend).unwrap();

            let terminal_chunk = Rect::new(0, 0, 46, 20);

            let base_chunk = child_window_chunk(50, 50, terminal_chunk);

            let mut w = Stack::builder()
                .widget(
                    List::builder()
                        .widget_config(
                            &WidgetConfig::builder()
                                .title("List-0")
                                .disable_focus()
                                .build(),
                        )
                        .build()
                        .into(),
                )
                .widget(
                    List::builder()
                        .widget_config(
                            &WidgetConfig::builder()
                                .title("List-1")
                                .disable_focus()
                                .build(),
                        )
                        .build()
                        .into(),
                )
                .widget(
                    List::builder()
                        .widget_config(
                            &WidgetConfig::builder()
                                .title("List-2")
                                .disable_focus()
                                .build(),
                        )
                        .build()
                        .into(),
                )
                .widget(
                    List::builder()
                        .widget_config(
                            &WidgetConfig::builder()
                                .title("List-3")
                                .disable_focus()
                                .build(),
                        )
                        .build()
                        .into(),
                )
                .widget(
                    List::builder()
                        .widget_config(
                            &WidgetConfig::builder()
                                .title("List-4")
                                .disable_focus()
                                .build(),
                        )
                        .build()
                        .into(),
                )
                .build();

            w.update_chunk(base_chunk);

            terminal
                .draw(|f| {
                    w.render(f, true);
                })
                .unwrap();

            let expected = Buffer::with_lines::<&'static str>(vec![
                "                                              ",
                "                                              ",
                "                                              ",
                "               ┌─── List-4 ──────────┐        ",
                "             ┌─── List-3 ──────────┐ │        ",
                "           ┌─── List-2 ──────────┐ │ │        ",
                "         ┌─── List-1 ──────────┐ │ │ │        ",
                "       ┌─── List-0 ──────────┐ │ │ │ │        ",
                "       │                     │ │ │ │ │        ",
                "       │                     │ │ │ │ │        ",
                "       │                     │ │ │ │ │        ",
                "       │                     │ │ │ │ │        ",
                "       │                     │ │ │ │─┘        ",
                "       │                     │ │ │─┘          ",
                "       │                     │ │─┘            ",
                "       │                     │─┘              ",
                "       └─────────────────────┘                ",
                "                                              ",
                "                                              ",
                "                                              ",
            ]);

            terminal.backend().assert_buffer(&expected)
        }

        // #[test]
        // fn sample() {
        //     let backend = TestBackend::new(30, 10);
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
