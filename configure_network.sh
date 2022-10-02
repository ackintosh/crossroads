#!/bin/bash

set -Eeuo pipefail

# NOTE:
# These operations require privilege so we can't run them in Dockerfile.
# We should run this script with `docker[-compose] run --privileged`.
if [ $UID -ne 0 ]; then
  echo "Root privileges are required"
  exit 1;
fi

if [ `ip netns show | wc -l` -gt 1 ]; then
  echo "Network namespaces have already configured."
  exit;
fi

echo "Started to configure network namespaces."

# #############################################################################
echo "Create namespaces..."
# #############################################################################
ip netns add host1
ip netns add router1
ip netns add router2
ip netns add host2

# #############################################################################
echo "Create links..."
# #############################################################################
ip link add name host1-router1 type veth peer name router1-host1
ip link add name router1-router2 type veth peer name router2-router1
ip link add name router2-host2 type veth peer name host2-router2

# #############################################################################
echo "Connect the links to the namespaces"
# #############################################################################
ip link set host1-router1 netns host1
ip link set router1-host1 netns router1
ip link set router1-router2 netns router1
ip link set router2-router1 netns router2
ip link set router2-host2 netns router2
ip link set host2-router2 netns host2

# #############################################################################
echo "Configure network interfaces"
# #############################################################################
echo " === host1 ==="
ip netns exec host1 ip addr add 192.168.1.2/24 dev host1-router1
ip netns exec host1 ip link set host1-router1 up
ip netns exec host1 ethtool -K host1-router1 rx off tx off
ip netns exec host1 ip route add default via 192.168.1.1

echo " === router1 ==="
ip netns exec router1 ip addr add 192.168.1.1/24 dev router1-host1
ip netns exec router1 ip link set router1-host1 up
ip netns exec router1 ethtool -K router1-host1 rx off tx off
ip netns exec router1 ip addr add 192.168.0.1/24 dev router1-router2
ip netns exec router1 ip link set router1-router2 up
ip netns exec router1 ethtool -K router1-router2 rx off tx off
ip netns exec router1 ip route add default via 192.168.0.2
# Disabling ip_forward on router1 because forwarding here will be done by `Crossroads`. ;-)
ip netns exec router1 sysctl -w net.ipv4.ip_forward=0

echo " === router2 ==="
ip netns exec router2 ip addr add 192.168.0.2/24 dev router2-router1
ip netns exec router2 ip link set router2-router1 up
ip netns exec router2 ethtool -K router2-router1 rx off tx off
ip netns exec router2 ip route add default via 192.168.0.1
ip netns exec router2 ip addr add 192.168.2.1/24 dev router2-host2
ip netns exec router2 ip link set router2-host2 up
ip netns exec router2 ethtool -K router2-host2 rx off tx off
ip netns exec router2 sysctl -w net.ipv4.ip_forward=1

echo " === host2 ==="
ip netns exec host2 ip addr add 192.168.2.2/24 dev host2-router2
ip netns exec host2 ip link set host2-router2 up
ip netns exec host2 ethtool -K host2-router2 rx off tx off
ip netns exec host2 ip route add default via 192.168.2.1

echo "Done."
echo "Run \`ip netns show\` to see the configured namespaces."

