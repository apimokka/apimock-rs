# SUMMARY

- [Home](./README.md)

---

- [User Guide](./user-guide/README.md)
    - [Getting started](./user-guide/getting-started/README.md)
        - [File-based routing](./user-guide/getting-started/file-based-routing.md)
        - [Root configuration](./user-guide/getting-started/root-configuration.md)
        - [Rule-based routing 1](./user-guide/getting-started/rule-based-routing-1.md)
        - [Rule-based routing 2](./user-guide/getting-started/rule-based-routing-2.md)
        - [TOML configuration](./user-guide/toml-configuration.md)
    - [Examples](./user-guide/examples/README.md)
        - [Combining conditions 1](./user-guide/examples/combining-conditions-1.md)
        - [Combining conditions 2](./user-guide/examples/combining-conditions-2.md)
        - [Operators](./user-guide/examples/operators.md)
    - [FAQ](./user-guide/faq.md)
- [Wrapping Up](./user-guide/conclusion.md)

---

- [Advanced Topics](./advanced-topics/README.md)
    - [Response decision flow](./advanced-topics/response-decision-flow.md)
    - [Rule set config structure](./advanced-topics/rule-set-config-structure/README.md)
        - [`prefix` table](./advanced-topics/rule-set-config-structure/prefix.md)
        - [`rules` array of tables](./advanced-topics/rule-set-config-structure/rules/README.md)
            - [`when` table](./advanced-topics/rule-set-config-structure/rules/when.md)
            - [`respond` table](./advanced-topics/rule-set-config-structure/rules/respond.md)
    - [Configuration overview](./advanced-topics/configuration-overview.md)
    - [Middleware 1](./advanced-topics/middleware-basics-with-rhai-scripts.md)
    - [Middleware 2](./advanced-topics/middleware-map-based-response-variation.md)

---

- [Technical Referrence](./technical-referrence/README.md)
    - [Vision and Goals](./technical-referrence/vision-and-goals.md)
    - [Architecture](./technical-referrence/architecture.md)
    - [Design](./technical-referrence/design/README.md)
        - [Server](./technical-referrence/design/server/README.md)
        - [Response](./technical-referrence/design/response/README.md)
            - [Headers](./technical-referrence/design/response/headers/README.md)
                - [CORS](./technical-referrence/design/response/headers/cors.md)
                - [OPTIONS](./technical-referrence/design/response/headers/options.md)
