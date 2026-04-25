//! pre-defined variables are available:
//! - url_path: request url path
//! - body: request body json value defined only when exists
//! 
//! (ref) The Rhai Book: https://rhai.rs/book/language/statements.html

// // case `cargo run`:
// // let returned_json_file_path = "examples/config/tests/apimock-rule-set-data/middleware-test.json";
// // case `cargo test`:
// let returned_json_file_path = "apimock-rule-set-data/middleware-test.json";
// 
// // print(url_path);
// if url_path == "/middleware-test" {
//     return returned_json_file_path;
// }
// 
// if is_def_var("body") {
//     // print(body);
//     switch (url_path) {
//         "/middleware-test/dummy" if body.middleware == "isHere" => {
//             // exit() is useful when run in fn (here, equivalent to return statement)
//             exit(returned_json_file_path);
//         },
//         _ => ()
//     }
// }
// 
// return;
