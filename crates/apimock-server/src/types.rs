use hyper::body::Bytes;

use std::convert::Infallible;

pub type BoxBody = http_body_util::combinators::BoxBody<Bytes, Infallible>;
