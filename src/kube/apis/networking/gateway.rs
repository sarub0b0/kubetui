#![allow(unused_imports)]

use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, LabelSelectorRequirement};

pub mod v1;
pub mod v1alpha2;
pub mod v1beta1;

impl From<v1::GatewayListenersAllowedRoutesNamespacesSelector> for LabelSelector {
    fn from(value: v1::GatewayListenersAllowedRoutesNamespacesSelector) -> Self {
        Self {
            match_expressions: value.match_expressions.map(|exprs| {
                exprs
                    .into_iter()
                    .map(LabelSelectorRequirement::from)
                    .collect()
            }),
            match_labels: value.match_labels,
        }
    }
}

impl From<v1::GatewayListenersAllowedRoutesNamespacesSelectorMatchExpressions>
    for LabelSelectorRequirement
{
    fn from(value: v1::GatewayListenersAllowedRoutesNamespacesSelectorMatchExpressions) -> Self {
        Self {
            key: value.key,
            operator: value.operator,
            values: value.values,
        }
    }
}

impl From<v1beta1::GatewayListenersAllowedRoutesNamespacesSelector> for LabelSelector {
    fn from(value: v1beta1::GatewayListenersAllowedRoutesNamespacesSelector) -> Self {
        Self {
            match_expressions: value.match_expressions.map(|exprs| {
                exprs
                    .into_iter()
                    .map(LabelSelectorRequirement::from)
                    .collect()
            }),
            match_labels: value.match_labels,
        }
    }
}

impl From<v1beta1::GatewayListenersAllowedRoutesNamespacesSelectorMatchExpressions>
    for LabelSelectorRequirement
{
    fn from(
        value: v1beta1::GatewayListenersAllowedRoutesNamespacesSelectorMatchExpressions,
    ) -> Self {
        Self {
            key: value.key,
            operator: value.operator,
            values: value.values,
        }
    }
}
