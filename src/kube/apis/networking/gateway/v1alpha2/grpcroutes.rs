// WARNING: generated by kopium - manual changes will be overwritten
// kopium command: kopium --api-version=v1alpha2 --schema=disabled -f -
// kopium version: 0.17.2

use kube::CustomResource;
use serde::{Serialize, Deserialize};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;

#[derive(CustomResource, Serialize, Deserialize, Clone, Debug)]
#[kube(group = "gateway.networking.k8s.io", version = "v1alpha2", kind = "GRPCRoute", plural = "grpcroutes")]
#[kube(namespaced)]
#[kube(status = "GRPCRouteStatus")]
#[kube(schema = "disabled")]
pub struct GRPCRouteSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostnames: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "parentRefs")]
    pub parent_refs: Option<Vec<GRPCRouteParentRefs>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<GRPCRouteRules>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteParentRefs {
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
pub struct GRPCRouteRules {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "backendRefs")]
    pub backend_refs: Option<Vec<GRPCRouteRulesBackendRefs>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<GRPCRouteRulesFilters>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matches: Option<Vec<GRPCRouteRulesMatches>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesBackendRefs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<GRPCRouteRulesBackendRefsFilters>>,
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
pub struct GRPCRouteRulesBackendRefsFilters {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "extensionRef")]
    pub extension_ref: Option<GRPCRouteRulesBackendRefsFiltersExtensionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "requestHeaderModifier")]
    pub request_header_modifier: Option<GRPCRouteRulesBackendRefsFiltersRequestHeaderModifier>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "requestMirror")]
    pub request_mirror: Option<GRPCRouteRulesBackendRefsFiltersRequestMirror>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "responseHeaderModifier")]
    pub response_header_modifier: Option<GRPCRouteRulesBackendRefsFiltersResponseHeaderModifier>,
    #[serde(rename = "type")]
    pub r#type: GRPCRouteRulesBackendRefsFiltersType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesBackendRefsFiltersExtensionRef {
    pub group: String,
    pub kind: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesBackendRefsFiltersRequestHeaderModifier {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<GRPCRouteRulesBackendRefsFiltersRequestHeaderModifierAdd>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<Vec<GRPCRouteRulesBackendRefsFiltersRequestHeaderModifierSet>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesBackendRefsFiltersRequestHeaderModifierAdd {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesBackendRefsFiltersRequestHeaderModifierSet {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesBackendRefsFiltersRequestMirror {
    #[serde(rename = "backendRef")]
    pub backend_ref: GRPCRouteRulesBackendRefsFiltersRequestMirrorBackendRef,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesBackendRefsFiltersRequestMirrorBackendRef {
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
pub struct GRPCRouteRulesBackendRefsFiltersResponseHeaderModifier {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<GRPCRouteRulesBackendRefsFiltersResponseHeaderModifierAdd>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<Vec<GRPCRouteRulesBackendRefsFiltersResponseHeaderModifierSet>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesBackendRefsFiltersResponseHeaderModifierAdd {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesBackendRefsFiltersResponseHeaderModifierSet {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GRPCRouteRulesBackendRefsFiltersType {
    ResponseHeaderModifier,
    RequestHeaderModifier,
    RequestMirror,
    ExtensionRef,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesFilters {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "extensionRef")]
    pub extension_ref: Option<GRPCRouteRulesFiltersExtensionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "requestHeaderModifier")]
    pub request_header_modifier: Option<GRPCRouteRulesFiltersRequestHeaderModifier>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "requestMirror")]
    pub request_mirror: Option<GRPCRouteRulesFiltersRequestMirror>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "responseHeaderModifier")]
    pub response_header_modifier: Option<GRPCRouteRulesFiltersResponseHeaderModifier>,
    #[serde(rename = "type")]
    pub r#type: GRPCRouteRulesFiltersType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesFiltersExtensionRef {
    pub group: String,
    pub kind: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesFiltersRequestHeaderModifier {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<GRPCRouteRulesFiltersRequestHeaderModifierAdd>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<Vec<GRPCRouteRulesFiltersRequestHeaderModifierSet>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesFiltersRequestHeaderModifierAdd {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesFiltersRequestHeaderModifierSet {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesFiltersRequestMirror {
    #[serde(rename = "backendRef")]
    pub backend_ref: GRPCRouteRulesFiltersRequestMirrorBackendRef,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesFiltersRequestMirrorBackendRef {
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
pub struct GRPCRouteRulesFiltersResponseHeaderModifier {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<GRPCRouteRulesFiltersResponseHeaderModifierAdd>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<Vec<GRPCRouteRulesFiltersResponseHeaderModifierSet>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesFiltersResponseHeaderModifierAdd {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesFiltersResponseHeaderModifierSet {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GRPCRouteRulesFiltersType {
    ResponseHeaderModifier,
    RequestHeaderModifier,
    RequestMirror,
    ExtensionRef,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesMatches {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<GRPCRouteRulesMatchesHeaders>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<GRPCRouteRulesMatchesMethod>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesMatchesHeaders {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub r#type: Option<GRPCRouteRulesMatchesHeadersType>,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GRPCRouteRulesMatchesHeadersType {
    Exact,
    RegularExpression,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteRulesMatchesMethod {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub r#type: Option<GRPCRouteRulesMatchesMethodType>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GRPCRouteRulesMatchesMethodType {
    Exact,
    RegularExpression,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteStatus {
    pub parents: Vec<GRPCRouteStatusParents>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteStatusParents {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<Condition>>,
    #[serde(rename = "controllerName")]
    pub controller_name: String,
    #[serde(rename = "parentRef")]
    pub parent_ref: GRPCRouteStatusParentsParentRef,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GRPCRouteStatusParentsParentRef {
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

