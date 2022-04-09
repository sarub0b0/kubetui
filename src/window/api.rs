use crossbeam::channel::Sender;

use std::{cell::RefCell, rc::Rc};

use crate::clipboard_wrapper::ClipboardContextWrapper;

use crate::event::{kubernetes::*, Event};

use crate::action::view_id;

use crate::tui_wrapper::{
    event::EventResult,
    tab::WidgetData,
    widget::{config::WidgetConfig, MultipleSelect, Text, Widget, WidgetTrait},
    Tab, Window,
};

pub struct ApiTabBuilder<'a> {
    title: &'a str,
    tx: &'a Sender<Event>,
    clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
}

pub struct ApiTab {
    pub tab: Tab<'static>,
    pub popup: Widget<'static>,
}

impl<'a> ApiTabBuilder<'a> {
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

    pub fn build(self) -> ApiTab {
        let api = self.api();

        ApiTab {
            tab: Tab::new(view_id::tab_api, self.title, [WidgetData::new(api)]),
            popup: self.popup().into(),
        }
    }

    fn api(&self) -> Text<'static> {
        let tx = self.tx.clone();

        let open_subwin = move |w: &mut Window| {
            tx.send(Event::Kube(Kube::GetAPIsRequest)).unwrap();
            w.open_popup(view_id::popup_api);
            EventResult::Nop
        };

        let builder = Text::builder()
            .id(view_id::tab_api_widget_api)
            .widget_config(&WidgetConfig::builder().title("API").build())
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
            .id(view_id::popup_api)
            .widget_config(&WidgetConfig::builder().title("API").build())
            .on_select(move |w, _| {
                let widget = w
                    .find_widget_mut(view_id::popup_api)
                    .as_mut_multiple_select();

                widget.toggle_select_unselect();

                if let Some(crate::tui_wrapper::widget::SelectedItem::Array(item)) =
                    widget.widget_item()
                {
                    let apis = item.iter().map(|i| i.item.to_string()).collect();
                    tx.send(Event::Kube(Kube::SetAPIsRequest(apis))).unwrap();
                }

                if widget.selected_items().is_empty() {
                    w.widget_clear(view_id::tab_api_widget_api)
                }

                EventResult::Nop
            })
            .build()
    }
}
