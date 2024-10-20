use std::collections::BTreeMap;

use crate::ui::widget::LiteralItem;

#[derive(Debug, Default)]
pub struct SelectItems {
    items: BTreeMap<LiteralItem, bool>,
}

impl SelectItems {
    pub fn update_items<T>(&mut self, items: impl Into<Vec<T>>)
    where
        T: Into<LiteralItem>,
    {
        let old = self.items.clone();

        self.items = items
            .into()
            .into_iter()
            .map(|i| (i.into(), false))
            .collect();

        old.iter().for_each(|(k, v)| {
            if let Some(value) = self.items.get_mut(k) {
                *value = *v;
            }
        })
    }

    pub fn toggle_select_unselect(&mut self, key: &LiteralItem) {
        if let Some(value) = self.items.get_mut(key) {
            *value = !*value;
        }
    }

    #[allow(dead_code)]
    pub fn items(&self) -> Vec<&LiteralItem> {
        self.items.keys().collect()
    }

    pub fn selected_items(&self) -> Vec<LiteralItem> {
        Self::filter_items(&self.items, true)
    }

    pub fn unselected_items(&self) -> Vec<LiteralItem> {
        Self::filter_items(&self.items, false)
    }

    pub fn select_all(&mut self) {
        self.items.values_mut().for_each(|v| *v = true);
    }

    pub fn unselect_all(&mut self) {
        self.items.values_mut().for_each(|v| *v = false);
    }

    fn filter_items(items: &BTreeMap<LiteralItem, bool>, is_active: bool) -> Vec<LiteralItem> {
        items
            .iter()
            .filter_map(|(k, v)| {
                if *v == is_active {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    #[allow(dead_code)]
    pub fn select(&mut self, key: &LiteralItem) {
        if let Some(value) = self.items.get_mut(key) {
            *value = true;
        }
    }

    #[allow(dead_code)]
    pub fn unselect(&mut self, key: &LiteralItem) {
        if let Some(value) = self.items.get_mut(key) {
            *value = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn select_unselect_and_selected_items() {
        let mut items = SelectItems::default();

        items.update_items([
            "Item 0".to_string(),
            "Item 1".to_string(),
            "Item 2".to_string(),
            "Item 3".to_string(),
            "Item 4".to_string(),
            "Item 5".to_string(),
        ]);

        items.select(&"Item 2".to_string().into());
        items.select(&"Item 5".to_string().into());
        items.select(&"Item 4".to_string().into());

        let expected: Vec<LiteralItem> = vec![
            "Item 2".to_string().into(),
            "Item 4".to_string().into(),
            "Item 5".to_string().into(),
        ];

        assert_eq!(items.selected_items(), expected);

        items.unselect(&"Item 2".to_string().into());

        let expected: Vec<LiteralItem> =
            vec!["Item 4".to_string().into(), "Item 5".to_string().into()];

        assert_eq!(items.selected_items(), expected);
    }

    #[test]
    fn update_items() {
        let mut items = SelectItems::default();

        items.update_items([
            "Item 0".to_string(),
            "Item 1".to_string(),
            "Item 2".to_string(),
            "Item 3".to_string(),
            "Item 4".to_string(),
            "Item 5".to_string(),
        ]);

        items.select(&"Item 2".to_string().into());
        items.select(&"Item 5".to_string().into());
        items.select(&"Item 4".to_string().into());

        let expected: Vec<LiteralItem> = vec![
            "Item 2".to_string().into(),
            "Item 4".to_string().into(),
            "Item 5".to_string().into(),
        ];

        assert_eq!(items.selected_items(), expected);

        items.update_items([
            "Item 0".to_string(),
            "Item 1".to_string(),
            "Item 2".to_string(),
        ]);

        let expected: Vec<LiteralItem> = vec!["Item 2".to_string().into()];

        assert_eq!(items.selected_items(), expected);
    }
}
