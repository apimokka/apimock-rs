use hyper::StatusCode;
use local_ip_address::list_afinet_netifas;

use crate::{
    constant::root_config_dir::listener::{GLOBAL, LOCALHOST},
    util::{
        http::{test_request::TestRequest, test_response::response_body_str},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn localhost_bound_same_loopback_request() {
    let port = localhost_listener_setup().await;
    let response = TestRequest::default("/", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "{\"hello\":\"index\"}");
}

#[tokio::test]
#[should_panic = "ConnectionRefused"]
async fn localhost_bound_another_loopback_request() {
    let port = localhost_listener_setup().await;
    let _ = TestRequest::default("/", port)
        .with_host("127.0.0.2")
        .send()
        .await;
}

#[tokio::test]
async fn global_bound_any_requests() {
    let port = global_listener_setup().await;

    let network_interfaces = list_afinet_netifas().unwrap();

    for (_name, ip) in network_interfaces.iter() {
        if !ip.is_ipv4() {
            continue;
        }

        // debug print:
        // println!("{}:\t{:?}", _name, ip);

        // localhost skipper for test case on lan addr such as 192.168.1.10:
        // if _name.as_str() == "lo" {
        //     continue;
        // }

        let response = TestRequest::default("/", port)
            .with_host(ip.to_string().as_str())
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        let body_str = response_body_str(response).await;
        assert_eq!(body_str.as_str(), "{\"hello\":\"index\"}");
    }
}

/// internal setup fn on https support config
async fn localhost_listener_setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(LOCALHOST);
    let port = test_setup.launch().await;
    port
}

/// internal setup fn on https support config
async fn global_listener_setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(GLOBAL);
    let port = test_setup.launch().await;
    port
}
