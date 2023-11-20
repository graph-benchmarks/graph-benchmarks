variable "vm_map" {
  type = map(object({
    name = string
    public_ip = optional(string)
  }))
}

variable "key_data" {
  type = string
  default = ""
}

output "key_data" {
  value = var.key_data
}

output "linux_virtual_machine_names" {
  value = [for s in keys(var.vm_map) : var.vm_map[s].name[*]]
}

output "linux_virtual_machine_ips" {
  value = [for s in keys(var.vm_map) : var.vm_map[s].public_ip[*]]
}