use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use ratatui::prelude::Constraint;

use crate::{
    clipboard::Clipboard,
    features::component_id::YAML_TAB_ID,
    message::Message,
    ui::{
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout, TabLayout},
        widget::Widget,
        Tab,
    },
};

use super::{
    dialogs::{kind::kind_dialog, name::name_dialog, not_found::not_found_dialog},
    widget::yaml_widget,
};

pub struct YamlTab {
    pub tab: Tab<'static>,
    pub kind_dialog: Widget<'static>,
    pub name_dialog: Widget<'static>,
    pub not_found_dialog: Widget<'static>,
}

impl YamlTab {
    pub fn new(
        title: &'static str,
        tx: &Sender<Message>,
        clipboard: &Option<Rc<RefCell<Clipboard>>>,
    ) -> Self {
        let yaml_widget = yaml_widget(tx, clipboard);

        let layout = TabLayout::new(
            |_| {
                NestedWidgetLayout::default().nested_widget_layout([NestedLayoutElement(
                    Constraint::Percentage(100),
                    LayoutElement::WidgetIndex(0),
                )])
            },
            Default::default(),
        );

        YamlTab {
            tab: Tab::new(YAML_TAB_ID, title, [yaml_widget], layout),
            kind_dialog: kind_dialog(tx),
            name_dialog: name_dialog(tx),
            not_found_dialog: not_found_dialog(),
        }
    }
}
