# Listener

The server listener is customizable with config file. In addition, some of them can be overwritten with startup parameter.

## Bound address

Each of them is available:

| Condition: `listener.ip_address` | Result |
| --- | --- |
| `127.0.0.1` | listens to localhost only |
| LAN address such as `192.168.1.10` | listens to LAN |
| `0.0.0.0` | listens to any globally |

### Example

Modification as below lets server listen to the external instead of localhost.
Note that there should be risk on security.

```diff
  # apimock.toml
  [listener]
- ip_address = "127.0.0.1"
+ ip_address = "0.0.0.0"
```
