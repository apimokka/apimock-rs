# Configuration overview

```mermaid
classDiagram
    direction LR
    class Config {
        + ListenerConfig listener (optional)
        + LogConfig log (optional)
        + ServiceConfig service
    }
    class ListenerConfig {
        + String ip_address
        + Integer port
        + TlsConfig tls_config (optional)
    }
    class TlsConfig {
        + String cert
        + String key
        + Integer port (optional)
    }
    class LogConfig {
        + VerboseConfig verbose
    }
    class VerboseConfig {
        + Boolean header
        + Boolean body
    }
    class ServiceConfig {
        + Array~RuleSet~ rule_sets
        + Array~RuleSet~ middlewares
        + String fallback_respond_dir
    }

    Config "1" o-- "0..1" ListenerConfig
    Config "1" o-- "0..1" LogConfig
    Config "1" o-- "1" ServiceConfig
    ListenerConfig "1" o-- "0..1" TlsConfig
    LogConfig "1" o-- "1" VerboseConfig
```

Here's an overview of the rule data structure in a nested Markdown format:

- `apimock.toml`
    - `[listener]` : Server listener.
        - `ip_address`: IP address.
        - `port`: Port for either HTTP or HTTPS.
        - `tls`
    - `[listener.tls]` : Server TLS/SSL settings.
        - `key`: Private key file path.
        - `cert`: Certificate file path.
        - `port`: Port for HTTPS.
    - `[log]` : Logger.
        - `verbose.header`: Verbose on request header.
        - `verbose.body`: Verbose on request body.
    - `[service]` : App service
        - **`rule_sets`:** Rule-based routing. The detail is [here](rule-set-config-structure/rules/).
        - `middlewares`
        - **`fallback_respond_dir`:** File-based routing base. The default is `.`, your current directory.
