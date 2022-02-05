use crossbeam::channel::Sender;
use std::{cell::RefCell, rc::Rc};

use crate::event::{kubernetes::*, Event};

use crate::action::view_id;
use crate::context::{Context, Namespace};

use crate::tui_wrapper::{
    event::EventResult,
    widget::{config::WidgetConfig, MultipleSelect, SingleSelect, Widget},
    Window,
};

pub struct ContextPopupBuilder<'a> {
    tx: &'a Sender<Event>,
    context: &'a Rc<RefCell<Context>>,
    namespaces: &'a Rc<RefCell<Namespace>>,
}

pub struct ContextPopup {
    pub context: Widget<'static>,
    pub single_namespace: Widget<'static>,
    pub multiple_namespaces: Widget<'static>,
}

impl<'a> ContextPopupBuilder<'a> {
    pub fn new(
        tx: &'a Sender<Event>,
        context: &'a Rc<RefCell<Context>>,
        namespaces: &'a Rc<RefCell<Namespace>>,
    ) -> Self {
        Self {
            tx,
            context,
            namespaces,
        }
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
        let namespaces = self.namespaces.clone();

        MultipleSelect::builder()
            .id(view_id::popup_ns)
            .widget_config(&WidgetConfig::builder().title("Namespace").build())
            .on_select(move |w: &mut Window, _| {
                let widget = w
                    .find_widget_mut(view_id::popup_ns)
                    .as_mut_multiple_select();

                widget.toggle_select_unselect();

                let mut items = widget.selected_items();
                if items.is_empty() {
                    items = vec!["None".to_string()];
                }

                tx.send(Event::Kube(Kube::SetNamespaces(items.clone())))
                    .unwrap();

                let mut ns = namespaces.borrow_mut();
                ns.selected = items;

                w.widget_clear(view_id::tab_pods_widget_logs);
                w.widget_clear(view_id::tab_configs_widget_raw_data);
                w.widget_clear(view_id::tab_event_widget_event);
                w.widget_clear(view_id::tab_apis_widget_apis);

                EventResult::Nop
            })
            .build()
    }

    fn context(&self) -> SingleSelect<'static> {
        let tx = self.tx.clone();
        let namespaces = self.namespaces.clone();
        let context = self.context.clone();

        SingleSelect::builder()
            .id(view_id::popup_ctx)
            .widget_config(&WidgetConfig::builder().title("Context").build())
            .on_select(move |w: &mut Window, v| {
                let item = v.to_string();

                tx.send(Event::Kube(Kube::SetContext(item.to_string())))
                    .unwrap();

                let mut ctx = context.borrow_mut();
                ctx.update(item);

                let mut ns = namespaces.borrow_mut();
                ns.selected = vec!["None".to_string()];

                w.close_popup();

                w.widget_clear(view_id::tab_pods_widget_logs);
                w.widget_clear(view_id::tab_configs_widget_raw_data);
                w.widget_clear(view_id::tab_event_widget_event);
                w.widget_clear(view_id::tab_apis_widget_apis);

                let widget = w
                    .find_widget_mut(view_id::popup_ns)
                    .as_mut_multiple_select();

                widget.unselect_all();

                let widget = w
                    .find_widget_mut(view_id::popup_apis)
                    .as_mut_multiple_select();

                widget.unselect_all();

                EventResult::Nop
            })
            .build()
    }

    fn single_namespace(&self) -> SingleSelect<'static> {
        let tx = self.tx.clone();
        let namespaces = self.namespaces.clone();

        SingleSelect::builder()
            .id(view_id::popup_single_ns)
            .widget_config(&WidgetConfig::builder().title("Namespace").build())
            .on_select(move |w: &mut Window, v| {
                let items = vec![v.to_string()];
                tx.send(Event::Kube(Kube::SetNamespaces(items.clone())))
                    .unwrap();

                let mut ns = namespaces.borrow_mut();
                ns.selected = items;

                w.close_popup();

                w.widget_clear(view_id::tab_pods_widget_logs);
                w.widget_clear(view_id::tab_configs_widget_raw_data);
                w.widget_clear(view_id::tab_event_widget_event);
                w.widget_clear(view_id::tab_apis_widget_apis);

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
