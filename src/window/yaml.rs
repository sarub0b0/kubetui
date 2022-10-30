use crossbeam::channel::Sender;
use std::{cell::RefCell, rc::Rc};

use crate::{
    action::view_id,
    clipboard_wrapper::Clipboard,
    event::{kubernetes::yaml::YamlRequest, Event},
    logger,
    tui_wrapper::{
        event::EventResult,
        tab::WidgetData,
        widget::Widget,
        widget::{config::WidgetConfig, SingleSelect, Text, WidgetTrait},
        Tab, Window,
    },
};

pub struct YamlTabBuilder<'a> {
    title: &'static str,
    tx: &'a Sender<Event>,
    clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
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
        clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
    ) -> Self {
        Self {
            title,
            tx,
            clipboard,
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

    fn main(&self) -> Text {
        let tx = self.tx.clone();

        let open_subwin = move |w: &mut Window| {
            tx.send(YamlRequest::APIs.into()).unwrap();
            w.open_popup(view_id::popup_yaml_kind);
            EventResult::Nop
        };

        let builder = Text::builder()
            .id(view_id::tab_yaml_widget_yaml)
            .widget_config(&WidgetConfig::builder().title("Yaml").build())
            .block_injection(|text: &Text, selected: bool| {
                let (index, size) = text.state();

                let mut config = text.widget_config().clone();

                *config.append_title_mut() = Some(format!(" [{}/{}]", index, size).into());

                config.render_block(text.focusable() && selected)
            })
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

        SingleSelect::builder()
            .id(view_id::popup_yaml_kind)
            .widget_config(&WidgetConfig::builder().title("Kind").build())
            .on_select(move |w, v| {
                logger!(info, "Select Item: {:?}", v);

                w.close_popup();

                tx.send(YamlRequest::Resource(v.item.to_string()).into())
                    .unwrap();

                w.open_popup(view_id::popup_yaml_name);

                EventResult::Nop
            })
            .build()
    }

    fn subwin_name(&self) -> SingleSelect<'static> {
        let tx = self.tx.clone();

        SingleSelect::builder()
            .id(view_id::popup_yaml_name)
            .widget_config(&WidgetConfig::builder().title("Name").build())
            .on_select(move |w, v| {
                logger!(info, "Select Item: {:?}", v);

                w.close_popup();

                v.metadata.as_ref().map_or(EventResult::Ignore, |metadata| {
                    metadata
                        .get("namespace")
                        .map_or(EventResult::Ignore, |namespace| {
                            metadata.get("kind").map_or(EventResult::Ignore, |kind| {
                                metadata.get("name").map_or(EventResult::Ignore, |name| {
                                    tx.send(
                                        YamlRequest::Yaml {
                                            kind: kind.to_string(),
                                            name: name.to_string(),
                                            namespace: namespace.to_string(),
                                        }
                                        .into(),
                                    )
                                    .unwrap();

                                    EventResult::Nop
                                })
                            })
                        })
                })
            })
            .build()
    }
}
