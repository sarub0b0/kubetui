use crossbeam::channel::Sender;
use k8s_openapi::{
    Resource,
    api::{
        core::v1::{Pod, Service},
        networking::v1::{Ingress, NetworkPolicy},
    },
};

use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::{NETWORK_DESCRIPTION_WIDGET_ID, NETWORK_WIDGET_ID},
        network::message::{NetworkRequest, NetworkRequestTargetParams},
    },
    kube::apis::networking::gateway::v1::{Gateway, HTTPRoute},
    message::Message,
    ui::{
        Window, WindowAction,
        event::EventResult,
        widget::{
            FilterForm, FilterFormTheme, Table, TableItem, TableTheme, Widget, WidgetBase,
            WidgetTheme, WidgetTrait as _,
        },
    },
};

pub fn network_widget(tx: &Sender<Message>, theme: WidgetThemeConfig) -> Widget<'static> {
    let tx = tx.clone();

    let widget_theme = WidgetTheme::from(theme.clone());
    let filter_theme = FilterFormTheme::from(theme.clone());
    let table_theme = TableTheme::from(theme.clone());

    let widget_base = WidgetBase::builder()
        .title("Network")
        .theme(widget_theme)
        .build();

    let filter_form = FilterForm::builder().theme(filter_theme).build();

    Table::builder()
        .id(NETWORK_WIDGET_ID)
        .widget_base(widget_base)
        .filter_form(filter_form)
        .theme(table_theme)
        .filtered_key("NAME")
        .block_injection(block_injection())
        .on_select(on_select(tx))
        .build()
        .into()
}

fn block_injection() -> impl Fn(&Table) -> WidgetBase {
    |table: &Table| {
        let index = if let Some(index) = table.state().selected() {
            index + 1
        } else {
            0
        };

        let mut base = table.widget_base().clone();

        *base.append_title_mut() = Some(format!(" [{}/{}]", index, table.items().len()).into());

        base
    }
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &TableItem) -> EventResult {
    move |w: &mut Window, v: &TableItem| {
        w.widget_clear(NETWORK_DESCRIPTION_WIDGET_ID);

        let Some(metadata) = v.metadata.as_ref() else {
            return EventResult::Ignore;
        };

        let Some(namespace) = metadata.get("namespace") else {
            return EventResult::Ignore;
        };

        let Some(name) = metadata.get("name") else {
            return EventResult::Ignore;
        };

        let Some(kind) = metadata.get("kind") else {
            return EventResult::Ignore;
        };

        let Some(version) = metadata.get("version") else {
            return EventResult::Ignore;
        };

        *(w.find_widget_mut(NETWORK_DESCRIPTION_WIDGET_ID)
            .widget_base_mut()
            .append_title_mut()) = Some((format!(" : {name}")).into());

        let request_data = NetworkRequestTargetParams {
            namespace: namespace.to_string(),
            name: name.to_string(),
            version: version.to_string(),
        };

        match kind.as_str() {
            Pod::KIND => {
                tx.send(NetworkRequest::Pod(request_data).into())
                    .expect("Failed to send NetworkRequest::Pod");
            }
            Service::KIND => {
                tx.send(NetworkRequest::Service(request_data).into())
                    .expect("Failed to send NetworkRequest::Service");
            }
            Ingress::KIND => {
                tx.send(NetworkRequest::Ingress(request_data).into())
                    .expect("Failed to send NetworkRequest::Ingress");
            }
            NetworkPolicy::KIND => {
                tx.send(NetworkRequest::NetworkPolicy(request_data).into())
                    .expect("Failed to send NetworkRequest::NetworkPolicy");
            }
            Gateway::KIND => {
                tx.send(NetworkRequest::Gateway(request_data).into())
                    .expect("Failed to send NetworkRequest::Gateway");
            }
            HTTPRoute::KIND => {
                tx.send(NetworkRequest::HTTPRoute(request_data).into())
                    .expect("Failed to send NetworkRequest::HTTPRoute");
            }
            _ => {
                unreachable!()
            }
        }

        EventResult::WindowAction(WindowAction::Continue)
    }
}
