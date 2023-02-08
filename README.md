# Network UDP test

A small tool that sends packets to a remote address that are expected to come back to us.

Before starting the tool, move the receiving interface into its own network namespace and assign the remote IP.
This will force your machine to use the default network configuration to reach that IP but when the trafic comes back through your interface,
it will be able to capture it.

```
# Create the network namespace
ip netns add network-tests
# Add that interface to this network namespace
ip link set <receiving-interface> netns network-tests
# Open a shell into that namespace to manage your interface
ip netns exec network-tests /bin/bash
# Up interface
ip link set <receiving-interface> up
# Assign the remote IP to your interface
ip a add 1.2.3.4 dev <receiving-interface>
```

Then launch the tool using root:
```
RUST_LOG=network_test_udp=info cargo run --release -- --rate 1 --size 16 --output /tmp/result --remote 1.2.3.4:1111 --network-namespace network-tests
```