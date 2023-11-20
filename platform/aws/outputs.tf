output "linux_virtual_machine_names" {
  value = [for s in aws_instance.graph-benchmarks-ec2 : s.tags.Name[*]]
}

output "linux_virtual_machine_ips" {
  value = [for s in aws_instance.graph-benchmarks-ec2 : s.public_ip[*]]
}