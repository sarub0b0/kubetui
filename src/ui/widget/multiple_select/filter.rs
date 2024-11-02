use std::ops::{Deref, DerefMut};

use crate::ui::widget::{InputForm, InputFormTheme, WidgetTheme};

use super::WidgetBase;

#[derive(Debug, Default)]
pub struct FilterFormTheme {
    widget_theme: WidgetTheme,
    input_form_theme: InputFormTheme,
}

impl FilterFormTheme {
    pub fn widget_theme(mut self, theme: impl Into<WidgetTheme>) -> Self {
        self.widget_theme = theme.into();
        self
    }

    pub fn input_form_theme(mut self, theme: impl Into<InputFormTheme>) -> Self {
        self.input_form_theme = theme.into();
        self
    }
}

#[derive(Debug, Default)]
pub struct FilterFormBuilder {
    theme: FilterFormTheme,
}

impl FilterFormBuilder {
    pub fn theme(mut self, theme: impl Into<FilterFormTheme>) -> Self {
        self.theme = theme.into();
        self
    }

    pub fn build(self) -> FilterForm {
        let widget_base = WidgetBase::builder()
            .theme(self.theme.widget_theme)
            .title("Filter")
            .build();

        let input_form = InputForm::builder()
            .widget_base(widget_base)
            .theme(self.theme.input_form_theme)
            .build();

        FilterForm { input_form }
    }
}

#[derive(Debug)]
pub struct FilterForm {
    input_form: InputForm,
}

impl Default for FilterForm {
    fn default() -> Self {
        FilterFormBuilder::default().build()
    }
}

impl FilterForm {
    pub fn builder() -> FilterFormBuilder {
        FilterFormBuilder::default()
    }
}

impl Deref for FilterForm {
    type Target = InputForm;

    fn deref(&self) -> &Self::Target {
        &self.input_form
    }
}

impl DerefMut for FilterForm {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.input_form
    }
}
