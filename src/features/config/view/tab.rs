use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use ratatui::layout::{Constraint, Direction};

use crate::{
    clipboard::Clipboard,
    config::theme::WidgetThemeConfig,
    features::{
        component_id::CONFIG_TAB_ID,
        config::{ConfigColumns, ConfigLabelColumn},
    },
    message::Message,
    ui::{
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout, TabLayout},
        Tab,
    },
};

use crate::ui::widget::Widget;

use super::widgets::{
    config_columns_dialog,
    config_filter_help_widget,
    config_widget,
    raw_data_widget,
};

pub struct ConfigTab {
    pub tab: Tab<'static>,
    pub config_columns_dialog: Widget<'static>,
    pub config_filter_help_dialog: Widget<'static>,
}

impl ConfigTab {
    pub fn new(
        title: &'static str,
        tx: &Sender<Message>,
        clipboard: &Option<Rc<RefCell<Clipboard>>>,
        split_direction: Direction,
        default_columns: ConfigColumns,
        label_registry: Vec<ConfigLabelColumn>,
        theme: WidgetThemeConfig,
    ) -> Self {
        let error_theme = theme.error.clone().into();

        let config_widget = config_widget(tx, label_registry.clone(), theme.clone());
        let raw_data_widget = raw_data_widget(clipboard, theme.clone());
        let config_columns_dialog =
            config_columns_dialog(tx, default_columns, label_registry, theme.clone());
        let config_filter_help_dialog = config_filter_help_widget(theme);

        let layout = TabLayout::new(layout, split_direction);

        Self {
            tab: Tab::new(
                CONFIG_TAB_ID,
                title,
                [config_widget, raw_data_widget],
                layout,
            )
            .error_theme(error_theme),
            config_columns_dialog,
            config_filter_help_dialog,
        }
    }
}

fn layout(split_direction: Direction) -> NestedWidgetLayout {
    NestedWidgetLayout::default()
        .direction(split_direction)
        .nested_widget_layout([
            NestedLayoutElement(Constraint::Percentage(50), LayoutElement::WidgetIndex(0)),
            NestedLayoutElement(Constraint::Percentage(50), LayoutElement::WidgetIndex(1)),
        ])
}
