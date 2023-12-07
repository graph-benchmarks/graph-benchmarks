provider "aws" {
  region     = "eu-central-1"
  access_key = "AKIA3PT3ZCP4SWEAM3DM"
  secret_key = "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"
}

data "aws_vpc" "default" {
  default = true
}

resource "aws_security_group" "graph-benchmarks-sec" {
  name   = "graph-benchmarks-sg"
  vpc_id = data.aws_vpc.default.id

  ingress {
    description = "All ingress"
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_eip" "graph-benchmarks-eip" {
  for_each = var.vm_map
  domain   = "vpc"
}

resource "aws_eip_association" "graph-benchmarks-eip-assoc" {
  for_each      = var.vm_map
  instance_id   = aws_instance.graph-benchmarks-ec2[each.key].id
  allocation_id = aws_eip.graph-benchmarks-eip[each.key].id
}

resource "aws_instance" "graph-benchmarks-ec2" {
  for_each               = var.vm_map
  ami                    = "ami-06dd92ecc74fdfb36"
  instance_type          = "r6i.2xlarge"
  key_name               = aws_key_pair.graph-benchmarks.key_name
  vpc_security_group_ids = [aws_security_group.graph-benchmarks-sec.id]

  root_block_device {
    volume_size = 100
  }

  tags = {
    Name = each.value.name
  }
}