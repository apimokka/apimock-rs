#![allow(dead_code)]

// inner attribute above suppresses unused warns for pub fns
// as test mods are compiled separately in rust and therefore the compiler doesn’t consider external calls

#[path = "constant.rs"]
mod constant;
#[path = "util/file.rs"]
mod file;
#[path = "util/http.rs"]
pub mod http;
#[path = "util/test_setup.rs"]
pub mod test_setup;
#[path = "util/tls.rs"]
mod tls;
