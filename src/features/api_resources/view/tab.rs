use crossbeam::channel::Sender;
use ratatui::prelude::Constraint;

use std::{cell::RefCell, rc::Rc};

use crate::{
    clipboard::Clipboard,
    config::theme::WidgetThemeConfig,
    features::component_id::API_TAB_ID,
    message::Message,
    ui::{
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout, TabLayout},
        widget::Widget,
        Tab,
    },
};

use super::{dialog::dialog_widget, widget::api_widget};

pub struct ApiTab {
    pub tab: Tab<'static>,
    pub dialog: Widget<'static>,
}

impl ApiTab {
    pub fn new(
        title: &'static str,
        tx: &Sender<Message>,
        clipboard: &Option<Rc<RefCell<Clipboard>>>,
        theme: WidgetThemeConfig,
    ) -> Self {
        let api_widget = api_widget(tx, clipboard, theme.clone());

        let layout = TabLayout::new(
            |_| {
                NestedWidgetLayout::default().nested_widget_layout([NestedLayoutElement(
                    Constraint::Percentage(100),
                    LayoutElement::WidgetIndex(0),
                )])
            },
            Default::default(),
        );

        ApiTab {
            tab: Tab::new(API_TAB_ID, title, [api_widget], layout),
            dialog: dialog_widget(tx, theme),
        }
    }
}
