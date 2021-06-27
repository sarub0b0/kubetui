use http::header::{HeaderValue, ACCEPT};
use kube::api::Request;
use kube::Result;

const TABLE_REQUEST_HEADER: &str = "application/json;as=Table;v=v1;g=meta.k8s.io,application/json;as=Table;v=v1beta1;g=meta.k8s.io,application/json";

pub fn get_table_request(server_url: &str, path: &str) -> Result<http::Request<Vec<u8>>> {
    let request = Request::new(server_url);

    let mut request = request.get(path)?;

    request
        .headers_mut()
        .insert(ACCEPT, HeaderValue::from_static(TABLE_REQUEST_HEADER));

    #[cfg(feature = "logging")]
    ::log::debug!("HTTP request {:?}", request);

    Ok(request)
}

pub fn get_request(server_url: &str, path: &str) -> Result<http::Request<Vec<u8>>> {
    let request = Request::new(server_url);

    let mut request = request.get(&path)?;

    request
        .headers_mut()
        .insert(ACCEPT, HeaderValue::from_static("application/json"));

    #[cfg(feature = "logging")]
    ::log::debug!("HTTP request {:?}", request);

    Ok(request)
}
