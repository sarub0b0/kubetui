use std::ops::{Deref, DerefMut};

use crate::ui::widget::InputForm;

use super::WidgetBase;

#[derive(Debug)]
pub struct FilterForm {
    input_form: InputForm,
}

impl Default for FilterForm {
    fn default() -> Self {
        Self {
            input_form: InputForm::builder()
                .widget_base(WidgetBase::builder().title("Filter").build())
                .build(),
        }
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
