// WARNING: generated by kopium - manual changes will be overwritten
// kopium command: kopium --api-version=v1beta1 --schema=disabled -f -
// kopium version: 0.17.2

use kube::CustomResource;
use serde::{Serialize, Deserialize};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;

impl k8s_openapi::Resource for HTTPRoute {
    const API_VERSION: &'static str = "gateway.networking.k8s.io/v1beta1";

    const GROUP: &'static str = "gateway.networking.k8s.io";

    const KIND: &'static str = "HTTPRoute";

    const VERSION: &'static str = "v1beta1";

    const URL_PATH_SEGMENT: &'static str = "httproutes";

    type Scope = k8s_openapi::NamespaceResourceScope;
}

#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, Default)]
#[kube(group = "gateway.networking.k8s.io", version = "v1beta1", kind = "HTTPRoute", plural = "httproutes")]
#[kube(namespaced)]
#[kube(status = "HTTPRouteStatus")]
#[kube(derive = "Default")]
#[kube(schema = "disabled")]
pub struct HTTPRouteSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostnames: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "parentRefs")]
    pub parent_refs: Option<Vec<HTTPRouteParentRefs>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<HTTPRouteRules>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteParentRefs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "sectionName")]
    pub section_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRules {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "backendRefs")]
    pub backend_refs: Option<Vec<HTTPRouteRulesBackendRefs>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<HTTPRouteRulesFilters>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matches: Option<Vec<HTTPRouteRulesMatches>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeouts: Option<HTTPRouteRulesTimeouts>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesBackendRefs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<HTTPRouteRulesBackendRefsFilters>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesBackendRefsFilters {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "extensionRef")]
    pub extension_ref: Option<HTTPRouteRulesBackendRefsFiltersExtensionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "requestHeaderModifier")]
    pub request_header_modifier: Option<HTTPRouteRulesBackendRefsFiltersRequestHeaderModifier>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "requestMirror")]
    pub request_mirror: Option<HTTPRouteRulesBackendRefsFiltersRequestMirror>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "requestRedirect")]
    pub request_redirect: Option<HTTPRouteRulesBackendRefsFiltersRequestRedirect>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "responseHeaderModifier")]
    pub response_header_modifier: Option<HTTPRouteRulesBackendRefsFiltersResponseHeaderModifier>,
    #[serde(rename = "type")]
    pub r#type: HTTPRouteRulesBackendRefsFiltersType,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "urlRewrite")]
    pub url_rewrite: Option<HTTPRouteRulesBackendRefsFiltersUrlRewrite>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesBackendRefsFiltersExtensionRef {
    pub group: String,
    pub kind: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesBackendRefsFiltersRequestHeaderModifier {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<HTTPRouteRulesBackendRefsFiltersRequestHeaderModifierAdd>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<Vec<HTTPRouteRulesBackendRefsFiltersRequestHeaderModifierSet>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesBackendRefsFiltersRequestHeaderModifierAdd {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesBackendRefsFiltersRequestHeaderModifierSet {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesBackendRefsFiltersRequestMirror {
    #[serde(rename = "backendRef")]
    pub backend_ref: HTTPRouteRulesBackendRefsFiltersRequestMirrorBackendRef,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesBackendRefsFiltersRequestMirrorBackendRef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesBackendRefsFiltersRequestRedirect {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<HTTPRouteRulesBackendRefsFiltersRequestRedirectPath>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<HTTPRouteRulesBackendRefsFiltersRequestRedirectScheme>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "statusCode")]
    pub status_code: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesBackendRefsFiltersRequestRedirectPath {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "replaceFullPath")]
    pub replace_full_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "replacePrefixMatch")]
    pub replace_prefix_match: Option<String>,
    #[serde(rename = "type")]
    pub r#type: HTTPRouteRulesBackendRefsFiltersRequestRedirectPathType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HTTPRouteRulesBackendRefsFiltersRequestRedirectPathType {
    ReplaceFullPath,
    ReplacePrefixMatch,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HTTPRouteRulesBackendRefsFiltersRequestRedirectScheme {
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "https")]
    Https,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HTTPRouteRulesBackendRefsFiltersRequestRedirectStatusCode {
    #[serde(rename = "301")]
    r#_301,
    #[serde(rename = "302")]
    r#_302,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesBackendRefsFiltersResponseHeaderModifier {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<HTTPRouteRulesBackendRefsFiltersResponseHeaderModifierAdd>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<Vec<HTTPRouteRulesBackendRefsFiltersResponseHeaderModifierSet>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesBackendRefsFiltersResponseHeaderModifierAdd {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesBackendRefsFiltersResponseHeaderModifierSet {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HTTPRouteRulesBackendRefsFiltersType {
    RequestHeaderModifier,
    ResponseHeaderModifier,
    RequestMirror,
    RequestRedirect,
    #[serde(rename = "URLRewrite")]
    UrlRewrite,
    ExtensionRef,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesBackendRefsFiltersUrlRewrite {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<HTTPRouteRulesBackendRefsFiltersUrlRewritePath>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesBackendRefsFiltersUrlRewritePath {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "replaceFullPath")]
    pub replace_full_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "replacePrefixMatch")]
    pub replace_prefix_match: Option<String>,
    #[serde(rename = "type")]
    pub r#type: HTTPRouteRulesBackendRefsFiltersUrlRewritePathType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HTTPRouteRulesBackendRefsFiltersUrlRewritePathType {
    ReplaceFullPath,
    ReplacePrefixMatch,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesFilters {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "extensionRef")]
    pub extension_ref: Option<HTTPRouteRulesFiltersExtensionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "requestHeaderModifier")]
    pub request_header_modifier: Option<HTTPRouteRulesFiltersRequestHeaderModifier>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "requestMirror")]
    pub request_mirror: Option<HTTPRouteRulesFiltersRequestMirror>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "requestRedirect")]
    pub request_redirect: Option<HTTPRouteRulesFiltersRequestRedirect>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "responseHeaderModifier")]
    pub response_header_modifier: Option<HTTPRouteRulesFiltersResponseHeaderModifier>,
    #[serde(rename = "type")]
    pub r#type: HTTPRouteRulesFiltersType,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "urlRewrite")]
    pub url_rewrite: Option<HTTPRouteRulesFiltersUrlRewrite>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesFiltersExtensionRef {
    pub group: String,
    pub kind: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesFiltersRequestHeaderModifier {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<HTTPRouteRulesFiltersRequestHeaderModifierAdd>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<Vec<HTTPRouteRulesFiltersRequestHeaderModifierSet>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesFiltersRequestHeaderModifierAdd {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesFiltersRequestHeaderModifierSet {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesFiltersRequestMirror {
    #[serde(rename = "backendRef")]
    pub backend_ref: HTTPRouteRulesFiltersRequestMirrorBackendRef,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesFiltersRequestMirrorBackendRef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesFiltersRequestRedirect {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<HTTPRouteRulesFiltersRequestRedirectPath>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<HTTPRouteRulesFiltersRequestRedirectScheme>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "statusCode")]
    pub status_code: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesFiltersRequestRedirectPath {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "replaceFullPath")]
    pub replace_full_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "replacePrefixMatch")]
    pub replace_prefix_match: Option<String>,
    #[serde(rename = "type")]
    pub r#type: HTTPRouteRulesFiltersRequestRedirectPathType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HTTPRouteRulesFiltersRequestRedirectPathType {
    ReplaceFullPath,
    ReplacePrefixMatch,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HTTPRouteRulesFiltersRequestRedirectScheme {
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "https")]
    Https,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HTTPRouteRulesFiltersRequestRedirectStatusCode {
    #[serde(rename = "301")]
    r#_301,
    #[serde(rename = "302")]
    r#_302,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesFiltersResponseHeaderModifier {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<HTTPRouteRulesFiltersResponseHeaderModifierAdd>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<Vec<HTTPRouteRulesFiltersResponseHeaderModifierSet>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesFiltersResponseHeaderModifierAdd {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesFiltersResponseHeaderModifierSet {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HTTPRouteRulesFiltersType {
    RequestHeaderModifier,
    ResponseHeaderModifier,
    RequestMirror,
    RequestRedirect,
    #[serde(rename = "URLRewrite")]
    UrlRewrite,
    ExtensionRef,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesFiltersUrlRewrite {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<HTTPRouteRulesFiltersUrlRewritePath>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesFiltersUrlRewritePath {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "replaceFullPath")]
    pub replace_full_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "replacePrefixMatch")]
    pub replace_prefix_match: Option<String>,
    #[serde(rename = "type")]
    pub r#type: HTTPRouteRulesFiltersUrlRewritePathType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HTTPRouteRulesFiltersUrlRewritePathType {
    ReplaceFullPath,
    ReplacePrefixMatch,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesMatches {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<HTTPRouteRulesMatchesHeaders>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<HTTPRouteRulesMatchesMethod>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<HTTPRouteRulesMatchesPath>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "queryParams")]
    pub query_params: Option<Vec<HTTPRouteRulesMatchesQueryParams>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesMatchesHeaders {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub r#type: Option<HTTPRouteRulesMatchesHeadersType>,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HTTPRouteRulesMatchesHeadersType {
    Exact,
    RegularExpression,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HTTPRouteRulesMatchesMethod {
    #[serde(rename = "GET")]
    Get,
    #[serde(rename = "HEAD")]
    Head,
    #[serde(rename = "POST")]
    Post,
    #[serde(rename = "PUT")]
    Put,
    #[serde(rename = "DELETE")]
    Delete,
    #[serde(rename = "CONNECT")]
    Connect,
    #[serde(rename = "OPTIONS")]
    Options,
    #[serde(rename = "TRACE")]
    Trace,
    #[serde(rename = "PATCH")]
    Patch,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesMatchesPath {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub r#type: Option<HTTPRouteRulesMatchesPathType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HTTPRouteRulesMatchesPathType {
    Exact,
    PathPrefix,
    RegularExpression,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesMatchesQueryParams {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub r#type: Option<HTTPRouteRulesMatchesQueryParamsType>,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HTTPRouteRulesMatchesQueryParamsType {
    Exact,
    RegularExpression,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteRulesTimeouts {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "backendRequest")]
    pub backend_request: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteStatus {
    pub parents: Vec<HTTPRouteStatusParents>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteStatusParents {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<Condition>>,
    #[serde(rename = "controllerName")]
    pub controller_name: String,
    #[serde(rename = "parentRef")]
    pub parent_ref: HTTPRouteStatusParentsParentRef,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPRouteStatusParentsParentRef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "sectionName")]
    pub section_name: Option<String>,
}

