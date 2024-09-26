use std::ops::{Deref, DerefMut};

use crate::ui::widget::InputForm;

#[derive(Debug)]
pub struct FilterForm {
    input_form: InputForm,
    form_height: u16,
}

impl Default for FilterForm {
    fn default() -> Self {
        Self {
            input_form: InputForm::builder().prefix("FILTER: ").build(),
            form_height: 3,
        }
    }
}

impl FilterForm {
    pub fn form_height(&self) -> u16 {
        self.form_height
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
