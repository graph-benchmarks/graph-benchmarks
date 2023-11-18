terraform {
  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~>3.0"
    }
    azapi = {
      source  = "azure/azapi"
      version = "~>1.5"
    }
    random = {
      source  = "hashicorp/random"
      version = "~>3.0"
    }
  }
}

provider "azurerm" {
  features {}
}

resource "azurerm_resource_group" "graph-benchmarks" {
  name     = "graph-benchmarks"
  location = "westeurope"
}

resource "azurerm_linux_virtual_machine" "graph-benchmarks" {
  for_each              = var.vm_map
  name                  = each.value.name
  location              = azurerm_resource_group.graph-benchmarks.location
  resource_group_name   = azurerm_resource_group.graph-benchmarks.name
  network_interface_ids = [azurerm_network_interface.graph-benchmarks[each.key].id]
  size                  = "Standard_B2s"

  source_image_reference {
    publisher = "Canonical"
    offer     = "0001-com-ubuntu-server-jammy"
    sku       = "22_04-lts-gen2"
    version   = "22.04.202311010"
  }

  admin_ssh_key {
    username   = "azureadmin"
    public_key = jsondecode(azapi_resource_action.ssh_public_key_gen.output).publicKey
  }

  os_disk {
    disk_size_gb         = "30"
    caching              = "ReadWrite"
    storage_account_type = "Standard_LRS"
    name                 = "os_disk${each.value.name}"
  }

  computer_name  = each.value.name
  admin_username = "azureadmin"
}

# resource "null_resource" "setup-ec2" {
# 	depends_on = [ azurerm_public_ip.graph-benchmarks.ip_address ]
# 	provisioner "local-exec" {
# 		command = "ANSIBLE_HOST_KEY_CHECKING=False ansible-playbook -u ubuntu -i '${aws_instance.rproxy-ec2.public_ip},' --private-key ${var.private_key} -e 'pub_key=${var.public_key}' ec2/rathole-install.yaml"
# 	}
# }