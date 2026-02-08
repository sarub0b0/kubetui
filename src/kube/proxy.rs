use crate::logger;
use hyper_util::client::proxy::matcher::Matcher;

/// kube-rs が NO_PROXY を未サポートのため (kube-rs#1203)、
/// hyper-util の Matcher で NO_PROXY チェックを行い、
/// バイパス対象なら config.proxy_url をクリアする。
pub fn clear_proxy_if_no_proxy_matches(config: &mut kube::Config) {
    let Some(proxy_url) = &config.proxy_url else {
        logger!(debug, "proxy_url is not set; skipping NO_PROXY check");
        return;
    };

    logger!(
        debug,
        "proxy_url={}, cluster_url={}",
        proxy_url,
        config.cluster_url
    );

    let matcher = Matcher::from_env();

    if matcher.intercept(&config.cluster_url).is_none() {
        logger!(
            debug,
            "NO_PROXY matched cluster_url={}; clearing proxy_url",
            config.cluster_url
        );

        config.proxy_url = None;
    } else {
        logger!(
            debug,
            "NO_PROXY did not match cluster_url={}; keeping proxy_url",
            config.cluster_url
        );
    }
}
