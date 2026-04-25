use hyper::StatusCode;
use local_ip_address::list_afinet_netifas;

use std::net::IpAddr;

use crate::{
    constant::root_config_dir::listener::{
        IPV4_GLOBAL, IPV4_LOCALHOST, IPV6_GLOBAL, IPV6_LOCALHOST,
    },
    util::{
        http::{test_request::TestRequest, test_response::response_body_str},
        test_setup::TestSetup,
    },
};

#[tokio::test]
async fn ipv4_localhost_bound_same_loopback_request() {
    let port = ipv4_localhost_listener_setup().await;
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
async fn ipv4_localhost_bound_another_loopback_request() {
    let port = ipv4_localhost_listener_setup().await;
    let _ = TestRequest::default("/", port)
        .with_host("127.0.0.2")
        .send()
        .await;
}

#[tokio::test]
async fn ipv4_global_bound_any_requests() {
    let port = ipv4_global_listener_setup().await;

    let network_interfaces = list_afinet_netifas().unwrap();

    let network_interfaces = network_interfaces
        .iter()
        .filter(|(_, ip_addr)| ip_addr.is_ipv4())
        .collect::<Vec<&(String, IpAddr)>>();

    for (_name, ip_addr) in network_interfaces {
        // debug print:
        // println!("{}:\t{:?}", _name, ip_addr);

        // localhost skipper for test case on lan addr such as 192.168.1.10:
        // if ip_addr.is_loopback() {
        //     continue;
        // }

        let response = TestRequest::default("/", port)
            .with_host(ip_addr.to_string().as_str())
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

#[tokio::test]
async fn ipv6_localhost_bound_same_loopback_request() {
    let port = ipv6_localhost_listener_setup().await;
    let response = TestRequest::default("/", port)
        .with_host("[::1]")
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

#[tokio::test]
#[should_panic = "ConnectionRefused"]
async fn ipv6_localhost_bound_nonlocalhost_request() {
    let port = ipv6_localhost_listener_setup().await;
    let network_interfaces = list_afinet_netifas().unwrap();
    let ipv6_non_localhost_network_interface = network_interfaces
        .iter()
        .find(|(_, ip_addr)| ip_addr.is_ipv6() && !ip_addr.is_loopback());
    match ipv6_non_localhost_network_interface {
        Some((_, ip_addr)) => {
            let _ = TestRequest::default("/", port)
                .with_host(format!("[{}]", ip_addr.to_string()).as_str())
                .send()
                .await;
        }
        None => panic!("no global network interface"),
    }
}

#[tokio::test]
async fn ipv6_global_bound_any_requests() {
    let port = ipv6_global_listener_setup().await;

    let network_interfaces = list_afinet_netifas().unwrap();

    let network_interfaces = network_interfaces
        .iter()
        .filter(|(_, ip_addr)| ip_addr.is_ipv6())
        .collect::<Vec<&(String, IpAddr)>>();

    for (_name, ip_addr) in network_interfaces {
        // debug print:
        // println!("{}:\t{:?}", _name, ip_addr);

        // localhost skipper for test case on lan addr such as 192.168.1.10:
        // if ip_addr.is_loopback() {
        //     continue;
        // }

        // currently difficult to support test case on ipv6 link local address,
        // for scope id is required to be bound to addr
        if ip_addr.to_string().starts_with("fe80::") {
            continue;
        }

        let host = format!("[{}]", ip_addr);
        let response = TestRequest::default("/", port)
            .with_host(host.as_str())
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

/// internal setup fn on ipv4 localhost listener
async fn ipv4_localhost_listener_setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(IPV4_LOCALHOST);
    let port = test_setup.launch().await;
    port
}

/// internal setup fn on ipv4 global listener
async fn ipv4_global_listener_setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(IPV4_GLOBAL);
    let port = test_setup.launch().await;
    port
}

/// internal setup fn on on ipv6 localhost listener
async fn ipv6_localhost_listener_setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(IPV6_LOCALHOST);
    let port = test_setup.launch().await;
    port
}

/// internal setup fn on on ipv6 global listener
async fn ipv6_global_listener_setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(IPV6_GLOBAL);
    let port = test_setup.launch().await;
    port
}
