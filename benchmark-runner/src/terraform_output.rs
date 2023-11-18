use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    #[serde(rename = "key_data")]
    pub key_data: KeyData,
    #[serde(rename = "linux_virtual_machine_ips")]
    pub linux_virtual_machine_ips: LinuxVirtualMachineIps,
    #[serde(rename = "linux_virtual_machine_names")]
    pub linux_virtual_machine_names: LinuxVirtualMachineNames,
    #[serde(rename = "resource_group_name")]
    pub resource_group_name: ResourceGroupName,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyData {
    pub sensitive: bool,
    #[serde(rename = "type")]
    pub type_field: String,
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinuxVirtualMachineIps {
    pub sensitive: bool,
    #[serde(rename = "type")]
    pub type_field: (String, Vec<(String, Vec<String>)>),
    pub value: Vec<Vec<String>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinuxVirtualMachineNames {
    pub sensitive: bool,
    #[serde(rename = "type")]
    pub type_field: (String, Vec<(String, Vec<String>)>),
    pub value: Vec<Vec<String>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceGroupName {
    pub sensitive: bool,
    #[serde(rename = "type")]
    pub type_field: String,
    pub value: String,
}
