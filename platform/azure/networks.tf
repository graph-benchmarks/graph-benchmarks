resource "azurerm_virtual_network" "graph-benchmarks" {
  for_each            = var.vm_map
  name                = "${each.value.name}-vnet"
  address_space       = ["10.0.0.0/16"]
  location            = azurerm_resource_group.graph-benchmarks.location
  resource_group_name = azurerm_resource_group.graph-benchmarks.name
}

resource "azurerm_subnet" "graph-benchmarks" {
  for_each             = var.vm_map
  name                 = each.value.name
  resource_group_name  = azurerm_resource_group.graph-benchmarks.name
  virtual_network_name = azurerm_virtual_network.graph-benchmarks[each.key].name
  address_prefixes     = ["10.0.0.0/24"]
}

resource "azurerm_public_ip" "graph-benchmarks" {
  for_each            = var.vm_map
  name                = each.value.name
  location            = azurerm_resource_group.graph-benchmarks.location
  resource_group_name = azurerm_resource_group.graph-benchmarks.name
  allocation_method   = "Static"
}

resource "azurerm_network_interface" "graph-benchmarks" {
  for_each            = var.vm_map
  name                = each.value.name
  location            = azurerm_resource_group.graph-benchmarks.location
  resource_group_name = azurerm_resource_group.graph-benchmarks.name

  ip_configuration {
    name                          = each.value.name
    private_ip_address_allocation = "Dynamic"
    subnet_id                     = azurerm_subnet.graph-benchmarks[each.key].id
    public_ip_address_id          = azurerm_public_ip.graph-benchmarks[each.key].id
  }
}

resource "azurerm_network_security_group" "graph-benchmarks" {
  for_each            = var.vm_map
  name                = each.value.name
  location            = azurerm_resource_group.graph-benchmarks.location
  resource_group_name = azurerm_resource_group.graph-benchmarks.name

  security_rule {
    name                       = "22"
    priority                   = 100
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "*"
    source_address_prefix      = "*"
    destination_address_prefix = "*"
  }
}

resource "azurerm_subnet_network_security_group_association" "graph-benchmarks" {
  for_each                  = var.vm_map
  subnet_id                 = azurerm_subnet.graph-benchmarks[each.key].id
  network_security_group_id = azurerm_network_security_group.graph-benchmarks[each.key].id
}