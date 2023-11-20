use serde::{Serialize, Deserialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub network: Network,
    pub nodes: Nodes,
    pub software: Software,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Network {
    #[serde(rename = "control_ip")]
    pub control_ip: String,
    #[serde(rename = "dns_servers")]
    pub dns_servers: Vec<String>,
    #[serde(rename = "pod_cidr")]
    pub pod_cidr: String,
    #[serde(rename = "service_cidr")]
    pub service_cidr: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Nodes {
    pub control: Control,
    pub workers: Workers,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Control {
    pub cpu: usize,
    pub memory: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workers {
    pub count: usize,
    pub cpu: usize,
    pub memory: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Software {
    #[serde(rename = "box")]
    pub box_field: String,
}