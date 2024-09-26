use std::ops::{Deref, DerefMut};

use ratatui::widgets::Block;

use crate::ui::widget::{InputForm, WidgetBase};

#[derive(Debug)]
pub struct SearchForm {
    input_form: InputForm,
    form_height: u16,
}

impl Default for SearchForm {
    fn default() -> Self {
        Self {
            input_form: InputForm::builder()
                .widget_base(WidgetBase::builder().block(Block::default()).build())
                .prefix("Search: ")
                .build(),
            form_height: 1,
        }
    }
}

impl SearchForm {
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
