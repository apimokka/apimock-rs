pub const CONFIG_FILE_NAME: &str = "apimock.toml";
pub const CONFIG_TESTS_ROOT_DIR_PATH: &str = "examples/config/tests";
pub const DYN_ROUTE_DIR: &str = "apimock-dyn-route";

pub const DUMMY_BINARY_DATA: &[u8] = b"Q\xb0\xd6wE\xc6\xbc\xaa\x1a\x01\xbf\x9e\xb0\xf6\xac\xcd-\xe8\x8dDdummy\x97\x8d%.2\x10v)\xb5\xc6\x0b\x01\xcd\xdc4\xb9O%u\x8d";

pub mod root_config_dir {
    pub mod listener {
        pub const IPV4_LOCALHOST: &str = "listener/@bound-address/ipv4/localhost";
        pub const IPV4_GLOBAL: &str = "listener/@bound-address/ipv4/global";
        pub const IPV6_LOCALHOST: &str = "listener/@bound-address/ipv6/localhost";
        pub const IPV6_GLOBAL: &str = "listener/@bound-address/ipv6/global";
        pub const TLS: &str = "listener/tls";
    }

    pub const ERROR_RESPONSE: &str = "apimock-rule-sets/server/response/error_response";
    pub const FILE_RESPONSE: &str = "apimock-rule-sets/server/response/file_response";
    pub const RULE_SET_PREFIX: &str = "apimock-rule-sets/server/routing/rule_set/prefix";
    pub const RULE_WHEN_REQUEST_URL_PATH: &str =
        "apimock-rule-sets/server/routing/rule_set/rule/when/request/url_path";
    pub const RULE_WHEN_REQUEST_HTTP_METHOD: &str =
        "apimock-rule-sets/server/routing/rule_set/rule/when/request/http_method";
    pub const RULE_WHEN_REQUEST_HEADERS: &str =
        "apimock-rule-sets/server/routing/rule_set/rule/when/request/headers";
    pub const RULE_WHEN_REQUEST_BODY: &str =
        "apimock-rule-sets/server/routing/rule_set/rule/when/request/body";
    pub const RULE_WHEN_REQUEST_RULE_OP: &str =
        "apimock-rule-sets/server/routing/rule_set/rule/when/request/rule_op";
    pub const RULE_RESPOND: &str = "apimock-rule-sets/server/routing/rule_set/rule/respond";
    pub const MIDDLEWARE: &str = "apimock-middleware";
    pub const CONFIG_FREE_ENV: &str = "apimock-rule-sets/@extra-test-cases/config-free-env";
}

pub mod tls {
    pub const CERT_FILE_PATH: &str = "tests/fixtures/tmp/cert.pem";
    pub const KEY_FILE_PATH: &str = "tests/fixtures/tmp/key.pem";
}
