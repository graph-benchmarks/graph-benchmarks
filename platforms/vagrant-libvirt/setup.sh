#!/bin/bash

set -euxo pipefail

if [ ! -d /etc/systemd/resolved.conf.d ]; then
	sudo mkdir /etc/systemd/resolved.conf.d/
fi

# setup dns
cat <<EOF | sudo tee /etc/systemd/resolved.conf.d/dns_servers.conf
[Resolve]
DNS=${DNS_SERVERS}
EOF

echo "export PATH=\$PATH:/sbin" >> /home/vagrant/.bashrc
echo "export HOME=/home/vagrant" >> /home/vagrant/.bashrc

# disable swap
sudo swapoff -a

# keeps the swap off during reboot
(crontab -l 2>/dev/null; echo "@reboot /sbin/swapoff -a") | crontab - || true

cat /home/vagrant/.ssh/me.pub >> /home/vagrant/.ssh/authorized_keys
sudo apt-get update
sudo apt-get install -y apt-transport-https ca-certificates curl python3 cloud-utils
sudo rm /usr/lib/python3.*/EXTERNALLY-MANAGED

sudo growpart /dev/vda 1
sudo resize2fs /dev/vda1