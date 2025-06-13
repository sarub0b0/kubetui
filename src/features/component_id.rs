use paste::paste;

macro_rules! component_id {
    ($($id:ident),*) => {
        paste! {
            $(
                pub const [<$id:upper _ID>]: &str = stringify!($id);
            )*
        }
    };
}

component_id!(
    // tabs
    pod_tab,
    config_tab,
    event_tab,
    api_tab,
    network_tab,
    yaml_tab,
    // widgets
    pod_widget,
    pod_log_widget,
    pod_log_query_widget,
    config_widget,
    config_raw_data_widget,
    network_widget,
    network_description_widget,
    event_widget,
    api_widget,
    yaml_widget,
    // dialogs
    pod_columns_dialog,
    pod_log_query_help_dialog,
    context_dialog,
    single_namespace_dialog,
    multiple_namespaces_dialog,
    api_dialog,
    yaml_kind_dialog,
    yaml_name_dialog,
    yaml_not_found_dialog,
    help_dialog,
    yaml_dialog
);
