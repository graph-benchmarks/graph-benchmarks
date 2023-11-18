resource "azapi_resource_action" "ssh_public_key_gen" {
  type        = "Microsoft.Compute/sshPublicKeys@2022-11-01"
  resource_id = azapi_resource.ssh_public_key.id
  action      = "generateKeyPair"
  method      = "POST"

  response_export_values = ["publicKey", "privateKey"]
}

resource "azapi_resource" "ssh_public_key" {
  type      = "Microsoft.Compute/sshPublicKeys@2022-11-01"
  name      = "graph-benchmarks"
  location  = azurerm_resource_group.graph-benchmarks.location
  parent_id = azurerm_resource_group.graph-benchmarks.id
}

output "key_data" {
  value = jsondecode(azapi_resource_action.ssh_public_key_gen.output).privateKey
}