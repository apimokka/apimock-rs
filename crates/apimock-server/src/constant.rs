pub const CSV_RECORDS_DEFAULT_KEY: &str = "records";

const DEFAULT_ALLOWED_METHODS: &str = "GET, POST, PUT, DELETE, OPTIONS";
pub const DEFAULT_RESPONSE_HEADERS: &[(&str, &str)] = &[
    ("access-control-allow-headers", "*"),
    ("access-control-allow-methods", DEFAULT_ALLOWED_METHODS),
    ("access-control-max-age", "86400"),
    ("cache-control", "no-store"),
    ("connection", "keep-alive"),
    ("x-content-type-options", "nosniff"),
];

pub const ROOT_DIRECTORY_FILE_NAME: &str = "index";
