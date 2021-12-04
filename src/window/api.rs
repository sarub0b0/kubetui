use crossbeam::channel::Sender;

use std::{cell::RefCell, rc::Rc};
use tui_wrapper::{tab::WidgetData, widget::Widget, Tab};

use clipboard_wrapper::ClipboardContextWrapper;

use ::event::{kubernetes::*, Event};

use crate::action::view_id;

use tui_wrapper::{
    event::EventResult,
    widget::{config::WidgetConfig, MultipleSelect, Text, WidgetTrait},
    Window,
};

pub struct APIsTabBuilder<'a> {
    title: &'a str,
    tx: &'a Sender<Event>,
    clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
}

pub struct APIsTab {
    pub tab: Tab<'static>,
    pub popup: Widget<'static>,
}

impl<'a> APIsTabBuilder<'a> {
    pub fn new(
        title: &'static str,
        tx: &'a Sender<Event>,
        clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
    ) -> Self {
        Self {
            title,
            tx,
            clipboard,
        }
    }

    pub fn build(self) -> APIsTab {
        let apis = self.apis();

        APIsTab {
            tab: Tab::new(view_id::tab_apis, self.title, [WidgetData::new(apis)]),
            popup: self.popup().into(),
        }
    }

    fn apis(&self) -> Text<'static> {
        let tx = self.tx.clone();

        let open_subwin = move |w: &mut Window| {
            tx.send(Event::Kube(Kube::GetAPIsRequest)).unwrap();
            w.open_popup(view_id::popup_apis);
            EventResult::Nop
        };

        let builder = Text::builder()
            .id(view_id::tab_apis_widget_apis)
            .widget_config(&WidgetConfig::builder().title("APIs").build())
            .block_injection(|text: &Text, selected: bool| {
                let (index, _) = text.state().selected();

                let mut config = text.widget_config().clone();

                *config.append_title_mut() =
                    Some(format!(" [{}/{}]", index, text.rows_size()).into());

                config.render_block_with_title(text.focusable() && selected)
            })
            .action('/', open_subwin.clone())
            .action('f', open_subwin);

        if let Some(cb) = self.clipboard {
            builder.clipboard(cb.clone())
        } else {
            builder
        }
        .build()
    }

    fn popup(&self) -> MultipleSelect<'static> {
        let tx = self.tx.clone();

        MultipleSelect::builder()
            .id(view_id::popup_apis)
            .widget_config(&WidgetConfig::builder().title("APIs").build())
            .on_select(move |w, _| {
                let widget = w
                    .find_widget_mut(view_id::popup_apis)
                    .as_mut_multiple_select();

                widget.toggle_select_unselect();

                if let Some(item) = widget.widget_item() {
                    tx.send(Event::Kube(Kube::SetAPIsRequest(item.array())))
                        .unwrap();
                }

                if widget.selected_items().is_empty() {
                    w.widget_clear(view_id::tab_apis_widget_apis)
                }

                EventResult::Nop
            })
            .build()
    }
}
