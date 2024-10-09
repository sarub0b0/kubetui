use std::ops::{Deref, DerefMut};

use ratatui::{style::Style, widgets::Block};

use crate::ui::widget::{InputForm, InputFormTheme, WidgetBase, WidgetTheme};

#[derive(Debug, Default)]
pub struct SearchFormTheme {
    /// WidgetBase::base_style
    base_style: Style,

    input_form_theme: InputFormTheme,
}

impl SearchFormTheme {
    pub fn base_style(mut self, style: impl Into<Style>) -> Self {
        self.base_style = style.into();
        self
    }

    pub fn input_form_theme(mut self, theme: impl Into<InputFormTheme>) -> Self {
        self.input_form_theme = theme.into();
        self
    }
}

#[derive(Debug, Default)]
pub struct SearchFormBuilder {
    theme: SearchFormTheme,
}

impl SearchFormBuilder {
    pub fn theme(mut self, theme: impl Into<SearchFormTheme>) -> Self {
        self.theme = theme.into();
        self
    }

    pub fn build(self) -> SearchForm {
        let widget_theme = WidgetTheme::default().base_style(self.theme.base_style);

        let widget_base = WidgetBase::builder()
            .block(Block::default())
            .theme(widget_theme)
            .build();

        let input_form = InputForm::builder()
            .widget_base(widget_base)
            .theme(self.theme.input_form_theme)
            .prefix("Search: ")
            .build();

        SearchForm {
            input_form,
            form_height: 1,
        }
    }
}

#[derive(Debug)]
pub struct SearchForm {
    input_form: InputForm,
    form_height: u16,
}

impl Default for SearchForm {
    fn default() -> Self {
        SearchFormBuilder::default().build()
    }
}

impl SearchForm {
    pub fn builder() -> SearchFormBuilder {
        SearchFormBuilder::default()
    }

    pub fn form_height(&self) -> u16 {
        self.form_height
    }
}

impl Deref for SearchForm {
    type Target = InputForm;

    fn deref(&self) -> &Self::Target {
        &self.input_form
    }
}

impl DerefMut for SearchForm {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.input_form
    }
}
