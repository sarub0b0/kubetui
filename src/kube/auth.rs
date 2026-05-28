use kube::config::ExecInteractiveMode;

use crate::logger;

/// Force any exec credential plugin to run non-interactively.
///
/// kube-rs runs exec auth plugins (e.g. `gke-gcloud-auth-plugin`) with
/// **inherited** stdin/stderr whenever `interactiveMode` is anything but
/// `Never` (`auth_exec` in kube-client). When such a plugin writes to
/// stderr during a token refresh — which GKE plugins do on token expiry,
/// e.g. after the machine resumes from sleep — that output lands directly
/// on the terminal. Our TUI draws on the alternate screen via stdout and
/// never redirects stderr, so the plugin's stderr corrupts the display.
///
/// Forcing `Never` makes kube-rs capture the plugin's stderr instead: a
/// failing refresh comes back as an `Err`, which we surface through the
/// normal `NotifyError` path (shown inside the widget) rather than smeared
/// across the screen. Interactive prompting would corrupt a TUI anyway, so
/// non-interactive is the correct mode here — a token that needs a fresh
/// interactive login simply fails and tells the user to re-authenticate
/// outside kubetui.
pub fn force_non_interactive_exec(config: &mut kube::Config) {
    if let Some(exec) = config.auth_info.exec.as_mut() {
        logger!(
            debug,
            "forcing exec auth interactiveMode=Never (was {:?})",
            exec.interactive_mode
        );
        exec.interactive_mode = Some(ExecInteractiveMode::Never);
    }
}

#[cfg(test)]
mod tests {
    use kube::config::{AuthInfo, ExecConfig};

    use super::*;

    fn config_with_exec(interactive_mode: Option<ExecInteractiveMode>) -> kube::Config {
        let mut config = kube::Config::new("https://example.com".parse().unwrap());
        config.auth_info = AuthInfo {
            exec: Some(ExecConfig {
                api_version: None,
                command: Some("some-auth-plugin".to_string()),
                args: None,
                env: None,
                drop_env: None,
                interactive_mode,
                provide_cluster_info: false,
                cluster: None,
            }),
            ..Default::default()
        };
        config
    }

    #[test]
    fn sets_interactive_mode_to_never_when_unspecified() {
        let mut config = config_with_exec(None);

        force_non_interactive_exec(&mut config);

        assert_eq!(
            config.auth_info.exec.unwrap().interactive_mode,
            Some(ExecInteractiveMode::Never)
        );
    }

    #[test]
    fn overrides_an_interactive_mode_to_never() {
        let mut config = config_with_exec(Some(ExecInteractiveMode::IfAvailable));

        force_non_interactive_exec(&mut config);

        assert_eq!(
            config.auth_info.exec.unwrap().interactive_mode,
            Some(ExecInteractiveMode::Never)
        );
    }

    #[test]
    fn no_op_when_there_is_no_exec_auth() {
        let mut config = kube::Config::new("https://example.com".parse().unwrap());

        force_non_interactive_exec(&mut config);

        assert!(config.auth_info.exec.is_none());
    }
}
