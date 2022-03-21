#![allow(dead_code)]
#![allow(unused_imports)]

mod pod {
    use std::collections::BTreeMap;

    use anyhow::Result;
    use k8s_openapi::{api::core::v1::Pod, List};

    type FetchedPodList = List<Pod>;

    struct FetchPodClient<'a, C> {
        client: &'a C,
        namespace: &'a str,
        selector: BTreeMap<&'a str, &'a str>,
    }

    impl<'a, C> FetchPodClient<'a, C> {
        fn new(client: &'a C, namespace: &'a str, selector: BTreeMap<&'a str, &'a str>) -> Self {
            Self {
                client,
                namespace,
                selector,
            }
        }

        async fn fetch(&self) -> Result<FetchedPodList> {
            unimplemented!()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        mod fetch {
            #[tokio::test]
            async fn podリストを取得する() {
            }
            #[tokio::test]
            async fn エラーのときerrを返す() {
            }
        }
        mod filter {
            use super::*;

            #[ignore]
            #[test]
            fn 関連するpodのリストを生成する() {}

            #[ignore]
            #[test]
            fn 関連するpodがないときnoneを返す() {}
        }

        mod to_value {
            use super::*;

            #[ignore]
            #[test]
            fn podのリストからnameのリストをvalue型で返す() {}

            #[ignore]
            #[test]
            fn リストが空のときnoneを返す() {}
        }
    }
}
