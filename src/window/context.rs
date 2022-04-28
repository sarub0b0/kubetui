use crossbeam::channel::Sender;

use crate::event::kubernetes::context_message::ContextRequest;
use crate::event::{kubernetes::*, Event};

use crate::action::view_id;

use crate::tui_wrapper::{
    event::EventResult,
    widget::{config::WidgetConfig, MultipleSelect, SingleSelect, Widget},
    Window,
};

pub struct ContextPopupBuilder<'a> {
    tx: &'a Sender<Event>,
}

pub struct ContextPopup {
    pub context: Widget<'static>,
    pub single_namespace: Widget<'static>,
    pub multiple_namespaces: Widget<'static>,
}

impl<'a> ContextPopupBuilder<'a> {
    pub fn new(tx: &'a Sender<Event>) -> Self {
        Self { tx }
    }

    pub fn build(self) -> ContextPopup {
        ContextPopup {
            context: self.context().into(),
            single_namespace: self.single_namespace().into(),
            multiple_namespaces: self.multiple_namespaces().into(),
        }
    }

    fn multiple_namespaces(&self) -> MultipleSelect<'static> {
        let tx = self.tx.clone();

        MultipleSelect::builder()
            .id(view_id::popup_ns)
            .widget_config(&WidgetConfig::builder().title("Namespace").build())
            .on_select(move |w: &mut Window, _| {
                let widget = w
                    .find_widget_mut(view_id::popup_ns)
                    .as_mut_multiple_select();

                let mut items: Vec<String> = widget
                    .selected_items()
                    .iter()
                    .map(|i| i.item.to_string())
                    .collect();

                if items.is_empty() {
                    items = vec!["None".to_string()];
                }

                tx.send(Event::Kube(Kube::SetNamespacesRequest(items)))
                    .unwrap();

                w.widget_clear(view_id::tab_pod_widget_log);
                w.widget_clear(view_id::tab_config_widget_raw_data);
                w.widget_clear(view_id::tab_event_widget_event);
                w.widget_clear(view_id::tab_api_widget_api);

                EventResult::Nop
            })
            .build()
    }

    fn context(&self) -> SingleSelect<'static> {
        let tx = self.tx.clone();
        SingleSelect::builder()
            .id(view_id::popup_ctx)
            .widget_config(&WidgetConfig::builder().title("Context").build())
            .on_select(move |w: &mut Window, v| {
                let item = v.item.to_string();

                tx.send(ContextRequest::Set(item).into()).unwrap();

                w.close_popup();

                w.widget_clear(view_id::tab_pod_widget_log);
                w.widget_clear(view_id::tab_config_widget_raw_data);
                w.widget_clear(view_id::tab_event_widget_event);
                w.widget_clear(view_id::tab_api_widget_api);

                let widget = w
                    .find_widget_mut(view_id::popup_ns)
                    .as_mut_multiple_select();

                widget.unselect_all();

                let widget = w
                    .find_widget_mut(view_id::popup_api)
                    .as_mut_multiple_select();

                widget.unselect_all();

                EventResult::Nop
            })
            .build()
    }

    fn single_namespace(&self) -> SingleSelect<'static> {
        let tx = self.tx.clone();

        SingleSelect::builder()
            .id(view_id::popup_single_ns)
            .widget_config(&WidgetConfig::builder().title("Namespace").build())
            .on_select(move |w: &mut Window, v| {
                let items = vec![v.item.to_string()];
                tx.send(Event::Kube(Kube::SetNamespacesRequest(items)))
                    .unwrap();

                w.close_popup();

                w.widget_clear(view_id::tab_pod_widget_log);
                w.widget_clear(view_id::tab_config_widget_raw_data);
                w.widget_clear(view_id::tab_event_widget_event);
                w.widget_clear(view_id::tab_api_widget_api);

                let widget = w
                    .find_widget_mut(view_id::popup_ns)
                    .as_mut_multiple_select();

                widget.unselect_all();

                widget.select_item(v);

                EventResult::Nop
            })
            .build()
    }
}
