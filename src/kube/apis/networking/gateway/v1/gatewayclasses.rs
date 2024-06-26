// WARNING: generated by kopium - manual changes will be overwritten
// kopium command: kopium --api-version=v1 --schema=disabled -f -
// kopium version: 0.17.2

use kube::CustomResource;
use serde::{Serialize, Deserialize};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;

#[derive(CustomResource, Serialize, Deserialize, Clone, Debug)]
#[kube(group = "gateway.networking.k8s.io", version = "v1", kind = "GatewayClass", plural = "gatewayclasses")]
#[kube(status = "GatewayClassStatus")]
#[kube(schema = "disabled")]
pub struct GatewayClassSpec {
    #[serde(rename = "controllerName")]
    pub controller_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "parametersRef")]
    pub parameters_ref: Option<GatewayClassParametersRef>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GatewayClassParametersRef {
    pub group: String,
    pub kind: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GatewayClassStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<Condition>>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "supportedFeatures")]
    pub supported_features: Option<Vec<String>>,
}

