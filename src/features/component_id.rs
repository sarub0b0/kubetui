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
    list_tab,
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
    list_widget,
    yaml_widget,
    // popups
    pod_log_query_help_popup,
    context_popup,
    single_namespace_popup,
    multiple_namespaces_popup,
    list_popup,
    yaml_kind_popup,
    yaml_name_popup,
    yaml_not_found_popup,
    help_popup,
    yaml_popup
);
