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

        #[test]
        fn nameリストに含まれるservice名のvalueを返す() {}

        #[test]
        fn nameリストに含まれるserviceがないときnoneを返す() {}

        #[test]
        fn エラーがでたときerrを返す() {}
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
