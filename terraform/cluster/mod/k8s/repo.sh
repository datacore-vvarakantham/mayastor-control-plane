#!/bin/bash
set -xeuo pipefail

RUNTIME="${kube_runtime}"
DISTRO=$(cat /etc/os-release | awk '/^NAME="/ {print $1}' | awk -F\" '{print $2}')
if [ ! "$DISTRO" = "Ubuntu" ]; then
  echo "Script only supports Ubuntu"
  exit 1
fi
if [ -n "${kube_version}" ]; then
  KUBE_VERSION="=${kube_version}"
fi
KVM_HOST_IP=$(ip -f inet addr show ens3 | awk '/inet / {print $2}' | awk -F. '{print $1"."$2"."$3".1"}')
echo "$KVM_HOST_IP kvmhost" | sudo tee -a /etc/hosts

if [ "$RUNTIME" = "crio" ]; then
  # Add Cri-o repo
  OS="xUbuntu_$(lsb_release -rs)"
  VERSION=1.26
  sudo add-apt-repository --yes "deb https://download.opensuse.org/repositories/devel:/kubic:/libcontainers:/stable/$OS/ /"
  sudo add-apt-repository --yes "deb http://download.opensuse.org/repositories/devel:/kubic:/libcontainers:/stable:/cri-o:/$VERSION/$OS/ /"
  curl -L https://download.opensuse.org/repositories/devel:kubic:libcontainers:stable:cri-o:$VERSION/$OS/Release.key | sudo apt-key add -
  curl -L https://download.opensuse.org/repositories/devel:/kubic:/libcontainers:/stable/$OS/Release.key | sudo apt-key add -
elif [ "$RUNTIME" = "containerd" ]; then
  # Add Docker repo
  curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -
  sudo add-apt-repository --yes "deb [arch=amd64] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable"
fi

curl -s https://packages.cloud.google.com/apt/doc/apt-key.gpg | sudo apt-key add -
cat <<EOF | sudo tee /etc/apt/sources.list.d/kubernetes.list
deb https://apt.kubernetes.io/ kubernetes-xenial main
EOF

sudo apt-get update

sudo apt-get install -y apt-transport-https curl ca-certificates software-properties-common

# Install k8s
sudo apt-get install -y -o Options::=--force-confdef \
  -o Dpkg::Options::=--force-confnew kubelet$KUBE_VERSION kubeadm$KUBE_VERSION kubectl$KUBE_VERSION
sudo apt-mark hold kubelet kubeadm kubectl

# Configure persistent loading of modules
sudo tee /etc/modules-load.d/k8s.conf <<EOF
overlay
br_netfilter
EOF
# Ensure you load modules
sudo modprobe overlay
sudo modprobe br_netfilter

# Set up required sysctl params
sudo tee /etc/sysctl.d/kubernetes.conf<<EOF
net.bridge.bridge-nf-call-ip6tables = 1
net.bridge.bridge-nf-call-iptables = 1
net.ipv4.ip_forward = 1
EOF

# Setup required sysctl params, these persist across reboots.
sudo sysctl --system

# Install CRI-O
if [ "$RUNTIME" = "crio" ]; then
  sudo apt install -y cri-o cri-o-runc
elif [ "$RUNTIME" = "containerd" ]; then
  sudo apt install -y containerd.io

  # Configure containerd and start service
  sudo mkdir -p /etc/containerd
  containerd config default | sudo tee /etc/containerd/config.toml
  sudo sed -i 's/            SystemdCgroup = false/            SystemdCgroup = true/' /etc/containerd/config.toml
  sudo sed -i 's/\[plugins."io.containerd.grpc.v1.cri".registry.mirrors\]/\[plugins."io.containerd.grpc.v1.cri".registry.mirrors\]\n        \[plugins."io.containerd.grpc.v1.cri".registry.mirrors."kvmhost:5000"\]\n          endpoint = \["http:\/\/kvmhost:5000"\]/g' /etc/containerd/config.toml
  sudo sed -i 's/\[plugins."io.containerd.grpc.v1.cri".registry.configs\]/\[plugins."io.containerd.grpc.v1.cri".registry.configs\]\n        \[plugins."io.containerd.grpc.v1.cri".registry.configs."kvmhost:5000".tls\]\n          insecure_skip_verify = true/g' /etc/containerd/config.toml
fi

# Start and enable Service
sudo systemctl daemon-reload

sudo systemctl restart $RUNTIME
sudo systemctl enable $RUNTIME
systemctl status $RUNTIME --no-pager
