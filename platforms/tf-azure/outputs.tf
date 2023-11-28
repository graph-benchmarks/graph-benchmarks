output "resource_group_name" {
  value = azurerm_resource_group.graph-benchmarks.name
}

output "linux_virtual_machine_names" {
  value = [for s in azurerm_linux_virtual_machine.graph-benchmarks : s.name[*]]
}

output "linux_virtual_machine_ips" {
  value = [for s in azurerm_public_ip.graph-benchmarks : s.ip_address[*]]
}