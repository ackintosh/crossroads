# Crossroads

```shell
docker-compose run --rm dev
```

Sending ARP requests

router1

```shell
ip netns exec router1 bash

RUST_LOG=debug cargo run
```

host1

```shell
ip netns exec host1 bash
arping -i host1-router1 192.168.1.1
ARPING 192.168.1.1
42 bytes from 0a:b5:c9:67:9f:88 (192.168.1.1): index=0 time=6.167 usec
42 bytes from 0a:b5:c9:67:9f:88 (192.168.1.1): index=1 time=4.542 usec
42 bytes from 0a:b5:c9:67:9f:88 (192.168.1.1): index=2 time=9.833 usec
...
...
```
