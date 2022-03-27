pub mod filter_by_name {
    use crate::event::kubernetes::{
        client::KubeClientRequest, network::description::related_resources::fetch::FetchClient,
    };

    use super::*;

    struct RelatedService<'a, C: KubeClientRequest> {
        client: FetchClient<'a, C>,
        names: Vec<String>,
    }

    mod filter {
        use super::*;
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        use anyhow::bail;
        use indoc::indoc;
        use k8s_openapi::{api::core::v1::Service, List};
        use mockall::predicate::eq;
        use pretty_assertions::assert_eq;
        use serde_yaml::Value;

        use crate::{event::kubernetes::client::mock::MockTestKubeClient, mock_expect};

        fn services() -> List<Service> {
            let yaml = indoc! {
                "
                items:
                  - metadata:
                      name: service-1
                  - metadata:
                      name: service-2
                  - metadata:
                      name: service-3
                "
            };

            serde_yaml::from_str(&yaml).unwrap()
        }

        #[tokio::test]
        async fn nameリストに含まれるservice名のvalueを返す() {
            let mut client = MockTestKubeClient::new();

            mock_expect!(
                client,
                request,
                List<Service>,
                eq("/api/v1/namespaces/default/services"),
                Ok(services())
            );

            let client = RelatedService::new(
                &client,
                "default",
                vec!["service-1".into(), "service-3".into()],
            );

            let result = client.related_resources().await.unwrap().unwrap();

            let expected = Value::from(vec!["service-1", "service-3"]);

            assert_eq!(result, expected);
        }

        #[tokio::test]
        async fn nameリストに含まれるserviceがないときnoneを返す() {
            let mut client = MockTestKubeClient::new();

            mock_expect!(
                client,
                request,
                List<Service>,
                eq("/api/v1/namespaces/default/services"),
                Ok(services())
            );

            let client = RelatedService::new(&client, "default", vec!["hoge".into()]);

            let result = client.related_resources().await.unwrap();

            assert_eq!(result.is_none(), true);
        }

        #[tokio::test]
        async fn エラーがでたときerrを返す() {
            let mut client = MockTestKubeClient::new();

            mock_expect!(
                client,
                request,
                List<Service>,
                eq("/api/v1/namespaces/default/services"),
                bail!("error")
            );

            let client = RelatedService::new(&client, "default", vec!["service-1".into()]);

            let result = client.related_resources().await;

            assert_eq!(result.is_err(), true);
        }
    }
}

pub mod filter_by_selector {
    use super::*;

    mod filter {
        use super::*;

        #[test]
        fn labelsにselectorの値を含むときそのserviceのリストを返す() {}

        #[test]
        fn labelsにselectorの値を含まないときnoneを返す() {}
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn labelsリストに含まれるservice名のvalueを返す() {}

        #[test]
        fn labelsリストに含まれるserviceがないときnoneを返す() {}

        #[test]
        fn エラーがでたときerrを返す() {}
    }
}
