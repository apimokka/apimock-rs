use hyper::StatusCode;

use crate::util::{
    http::{test_request::TestRequest, test_response::response_body_bytes},
    test_setup::TestSetup,
};

#[tokio::test]
async fn dyn_data_dir_image_png() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/image/image.png", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(response.headers().get("content-type").unwrap(), "image/png");

    let body_str = response_body_bytes(response).await;
    assert_eq!(
        body_str.as_ref(),
        b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0 \0\0\0 \x01\x03\0\0\0I\xb4\xe8\xb7\0\0\0\x03PLTE\xea\xf22\xedR\xba\x13\0\0\0\x0cIDAT\x08\xd7c`\x18\xdc\0\0\0\xa0\0\x01a%}G\0\0\0\0IEND\xaeB`\x82"
    );
}

#[tokio::test]
async fn dyn_data_dir_image_jpeg() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/image/image.jpg", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "image/jpeg"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(
        body_str.as_ref(),
        b"\xff\xd8\xff\xe0\0\x10JFIF\0\x01\x01\x01\0H\0H\0\0\xff\xdb\0C\0\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\xff\xdb\0C\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\xff\xc2\0\x11\x08\0 \0 \x03\x01\x11\0\x02\x11\x01\x03\x11\x01\xff\xc4\0\x15\0\x01\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\t\xff\xc4\0\x16\x01\x01\x01\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x06\t\xff\xda\0\x0c\x03\x01\0\x02\x10\x03\x10\0\0\x01\xad\x19\x07`\0\0\0\0\0\x0f\xff\xc4\0\x14\x10\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0@\xff\xda\0\x08\x01\x01\0\x01\x05\x02\x07\xff\xc4\0\x14\x11\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0@\xff\xda\0\x08\x01\x03\x01\x01?\x01\x07\xff\xc4\0\x14\x11\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0@\xff\xda\0\x08\x01\x02\x01\x01?\x01\x07\xff\xc4\0\x14\x10\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0@\xff\xda\0\x08\x01\x01\0\x06?\x02\x07\xff\xc4\0\x14\x10\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0@\xff\xda\0\x08\x01\x01\0\x01?!\x07\xff\xda\0\x0c\x03\x01\0\x02\0\x03\0\0\0\x10\0\0\0\0\0\0\xff\xc4\0\x14\x11\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0@\xff\xda\0\x08\x01\x03\x01\x01?\x10\x07\xff\xc4\0\x14\x11\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0@\xff\xda\0\x08\x01\x02\x01\x01?\x10\x07\xff\xc4\0\x14\x10\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0@\xff\xda\0\x08\x01\x01\0\x01?\x10\x07\xff\xd9"
    );
}

#[tokio::test]
async fn dyn_data_dir_image_gif() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/image/image.gif", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(response.headers().get("content-type").unwrap(), "image/gif");

    let body_str = response_body_bytes(response).await;
    assert_eq!(
        body_str.as_ref(),
        b"GIF87a \0 \0\x80\x01\0\xea\xf22\xff\xff\xff,\0\0\0\0 \0 \0\0\x02\x1e\x84\x8f\xa9\xcb\xed\x0f\xa3\x9c\xb4\xda\x8b\xb3\xde\xbc\xfb\x0f\x86\xe2H\x96\xe6\x89\xa6\xea\xca\xb6\xee\x0b\x9b\x05\0;"
    );
}

#[tokio::test]
async fn dyn_data_dir_image_bmp() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/image/image.bmp", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(response.headers().get("content-type").unwrap(), "image/bmp");

    let body_str = response_body_bytes(response).await;
    assert_eq!(
        body_str.as_ref(),
        [
            66, 77, 186, 0, 0, 0, 0, 0, 0, 0, 58, 0, 0, 0, 40, 0, 0, 0, 32, 0, 0, 0, 32, 0, 0, 0,
            1, 0, 1, 0, 0, 0, 0, 0, 128, 0, 0, 0, 19, 11, 0, 0, 19, 11, 0, 0, 1, 0, 0, 0, 1, 0, 0,
            0, 50, 242, 234, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
        ]
    );
}

#[tokio::test]
async fn dyn_data_dir_image_webp() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/image/image.webp", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "image/webp"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(
        body_str.as_ref(),
        [
            82, 73, 70, 70, 66, 0, 0, 0, 87, 69, 66, 80, 86, 80, 56, 32, 54, 0, 0, 0, 240, 1, 0,
            157, 1, 42, 32, 0, 32, 0, 0, 0, 0, 37, 148, 1, 216, 3, 240, 0, 9, 67, 42, 0, 0, 254,
            255, 202, 169, 191, 255, 255, 40, 243, 255, 40, 243, 255, 40, 243, 255, 242, 143, 63,
            255, 243, 211, 53, 230, 15, 240, 176, 0, 0
        ]
    );
}

// note: placed here although svg is text instead of binary
#[tokio::test]
async fn dyn_data_dir_image_svg() {
    let port = TestSetup::default().launch().await;

    let response = TestRequest::default("/binary/image/image.svg", port)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "image/svg+xml"
    );

    let body_str = response_body_bytes(response).await;
    assert_eq!(
        String::from_utf8(body_str.as_ref().to_vec()).unwrap(),
        r##"<?xml version="1.0" standalone="no"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 20010904//EN"
 "http://www.w3.org/TR/2001/REC-SVG-20010904/DTD/svg10.dtd">
<svg version="1.0" xmlns="http://www.w3.org/2000/svg"
 width="32.000000pt" height="32.000000pt" viewBox="0 0 32.000000 32.000000"
 preserveAspectRatio="xMidYMid meet">
<g transform="translate(0.000000,32.000000) scale(0.100000,-0.100000)"
fill="#000000" stroke="none">
</g>
</svg>
"##
    );
}
