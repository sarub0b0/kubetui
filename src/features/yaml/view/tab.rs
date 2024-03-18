use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use ratatui::prelude::Constraint;

use crate::{
    action::view_id,
    clipboard::Clipboard,
    message::Message,
    ui::{
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout},
        widget::Widget,
        Tab,
    },
};

use super::{
    popups::{kind::kind_popup, name::name_popup, not_found::not_found_popup},
    widget::yaml_widget,
};

pub struct YamlTab {
    pub tab: Tab<'static>,
    pub kind_popup: Widget<'static>,
    pub name_popup: Widget<'static>,
    pub not_found_popup: Widget<'static>,
}

impl YamlTab {
    pub fn new(
        title: &'static str,
        tx: &Sender<Message>,
        clipboard: &Option<Rc<RefCell<Clipboard>>>,
    ) -> Self {
        let yaml_widget = yaml_widget(tx, clipboard);

        let layout = NestedWidgetLayout::default().nested_widget_layout([NestedLayoutElement(
            Constraint::Percentage(100),
            LayoutElement::WidgetIndex(0),
        )]);

        YamlTab {
            tab: Tab::new(view_id::tab_yaml, title, [yaml_widget], layout),
            kind_popup: kind_popup(tx),
            name_popup: name_popup(tx),
            not_found_popup: not_found_popup(),
        }
    }
}
