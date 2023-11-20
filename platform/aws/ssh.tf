resource "tls_private_key" "private_key" {
  algorithm = "RSA"
  rsa_bits  = 4096
}

resource "aws_key_pair" "graph-benchmarks" {
  key_name   = "graph-benchmarks"
  public_key = tls_private_key.private_key.public_key_openssh
}

output "key_data" {
  sensitive = true
  value = tls_private_key.private_key.private_key_pem
}