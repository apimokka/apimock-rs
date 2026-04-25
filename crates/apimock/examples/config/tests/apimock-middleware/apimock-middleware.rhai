//! pre-defined variables are available:
//! - url_path: request url path
//! - body: request body json value defined only when exists
//! 
//! (ref) The Rhai Book: https://rhai.rs/book/language/statements.html

// case `cargo run`:
// let returned_json_file_path = "examples/config/full/apimock-rule-set-data/middleware-test.json";
// case `cargo test`:
let found_file_path = "@respond-dir/middleware-found.json";
let missing_file_path = "@respond-dir/middleware-missing.json";

// print(url_path);
if url_path == "/middleware-test/map/file-path" {
    return #{ "file_path": found_file_path };
}
else if url_path == "/middleware-test/map/json" {
    return #{ "json": "{ \"greetings\": \"Hello, world.\" }" };
}
else if url_path == "/middleware-test/map/text" {
    return #{ "text": "Hello, world." };
}
else if url_path == "/middleware-test" {
    return found_file_path;
}

if is_def_var("body") {
    // print(body);
    switch (url_path) {
        "/middleware-test/dummy" if body.middleware == "isHere" => {
            // exit() is useful when run in fn (here, equivalent to return statement)
            exit(missing_file_path);
        },
        _ => ()
    }
}

return;
