use crossbeam::channel::Sender;

use crate::tui_wrapper::{tab::WidgetData, widget::Widget, Tab};
use std::{cell::RefCell, rc::Rc};

use crate::clipboard_wrapper::ClipboardContextWrapper;

use crate::event::{kubernetes::*, Event};

use crate::action::view_id;
use crate::context::Namespace;

use crate::tui_wrapper::{
    event::EventResult,
    widget::{config::WidgetConfig, SingleSelect, Text, WidgetTrait},
    Window,
};

type YamlState = Rc<RefCell<(String, String)>>;

pub struct YamlTabBuilder<'a> {
    title: &'static str,
    tx: &'a Sender<Event>,
    namespaces: &'a Rc<RefCell<Namespace>>,
    clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
    state: YamlState,
}

pub struct YamlTab {
    pub tab: Tab<'static>,
    pub popup_kind: Widget<'static>,
    pub popup_name: Widget<'static>,
}

impl<'a> YamlTabBuilder<'a> {
    pub fn new(
        title: &'static str,
        tx: &'a Sender<Event>,
        namespaces: &'a Rc<RefCell<Namespace>>,
        clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
    ) -> Self {
        Self {
            title,
            tx,
            namespaces,
            clipboard,
            state: Default::default(),
        }
    }

    pub fn build(self) -> YamlTab {
        let yaml = self.main();
        YamlTab {
            tab: Tab::new(view_id::tab_yaml, self.title, [WidgetData::new(yaml)]),
            popup_kind: self.subwin_kind().into(),
            popup_name: self.subwin_name().into(),
        }
    }

    fn main(&self) -> Text<'static> {
        let tx = self.tx.clone();
        let state = self.state.clone();

        let open_subwin = move |w: &mut Window| {
            let mut state = state.borrow_mut();
            *state = (String::default(), String::default());

            tx.send(Event::Kube(Kube::YamlAPIsRequest)).unwrap();
            w.open_popup(view_id::popup_yaml_kind);
            EventResult::Nop
        };

        let builder = Text::builder()
            .id(view_id::tab_yaml_widget_yaml)
            .widget_config(&WidgetConfig::builder().title("Yaml").build())
            .block_injection(|text: &Text, selected: bool| {
                let (index, _) = text.state().selected();

                let mut config = text.widget_config().clone();

                *config.append_title_mut() =
                    Some(format!(" [{}/{}]", index, text.rows_size()).into());

                config.render_block_with_title(text.focusable() && selected)
            })
            .action('/', open_subwin.clone())
            .action('f', open_subwin)
            .wrap();

        if let Some(cb) = self.clipboard {
            builder.clipboard(cb.clone())
        } else {
            builder
        }
        .build()
    }

    fn subwin_kind(&self) -> SingleSelect<'static> {
        let tx = self.tx.clone();
        let state = self.state.clone();
        SingleSelect::builder()
            .id(view_id::popup_yaml_kind)
            .widget_config(&WidgetConfig::builder().title("Kind").build())
            .on_select(move |w, v| {
                #[cfg(feature = "logging")]
                ::log::info!("[subwin_yaml_kind] Select Item: {}", v);

                w.close_popup();

                let mut state = state.borrow_mut();
                state.0 = v.to_string();

                tx.send(Event::Kube(Kube::YamlResourceRequest(v.to_string())))
                    .unwrap();

                w.open_popup(view_id::popup_yaml_name);

                EventResult::Nop
            })
            .build()
    }

    fn subwin_name(&self) -> SingleSelect<'static> {
        let tx = self.tx.clone();
        let namespaces = self.namespaces.clone();
        let state = self.state.clone();

        SingleSelect::builder()
            .id(view_id::popup_yaml_name)
            .widget_config(&WidgetConfig::builder().title("Name").build())
            .on_select(move |w, v| {
                #[cfg(feature = "logging")]
                ::log::info!("[subwin_yaml_name] Select Item: {}", v);

                w.close_popup();

                let ns = &namespaces.borrow().selected;

                let value: Vec<&str> = v.split_whitespace().collect();

                let (name, ns) = if value.len() == 1 {
                    (value[0].to_string(), ns[0].to_string())
                } else {
                    (value[1].to_string(), value[0].to_string())
                };

                let state = state.borrow();

                let kind = state.0.to_string();

                tx.send(Event::Kube(Kube::YamlRawRequest(kind, name, ns)))
                    .unwrap();

                EventResult::Nop
            })
            .build()
    }
}
