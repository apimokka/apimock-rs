//! RFC 076 updated the `{"hello":"index"}` sanity-check assertions here
//! to the underlying `index.json` fixture's own raw bytes, since a
//! `.json` `file_path` is now served byte-for-byte. **Updated because
//! the bytes are now correct.**

use hyper::StatusCode;
use local_ip_address::list_afinet_netifas;

use std::{net::IpAddr, time::Duration};

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
    assert_eq!(body_str.as_str(), "{\n    \"hello\": \"index\"\n}");
}

/// The property under test is that the server does not answer a request
/// on a loopback address it wasn't told to bind — not the specific way
/// the connection fails to reach it. On Linux, `127.0.0.0/8` is
/// auto-bound to loopback, so an unconfigured alias like `127.0.0.2`
/// refuses the connection immediately. macOS's BSD network stack does
/// not extend that auto-binding, so the same connection attempt hangs
/// instead of refusing — both outcomes satisfy the property, so both
/// are accepted; only an actual response would be a failure. A short
/// connect timeout bounds the macOS case rather than leaving it to the
/// OS's own (tens-of-seconds) connect timeout.
#[tokio::test]
async fn ipv4_localhost_bound_another_loopback_request() {
    let port = ipv4_localhost_listener_setup().await;
    let result = TestRequest::default("/", port)
        .with_host("127.0.0.2")
        .with_connect_timeout(Duration::from_secs(3))
        .try_send()
        .await;

    // `is_connect()`/`is_timeout()` rather than a bare `is_err()`: the
    // property under test is "nothing is listening there", not merely
    // "something went wrong somewhere in the exchange" — a connection
    // that was established and then failed for an unrelated reason
    // must not count as a pass.
    let err = result.expect_err("expected the connection to be refused or time out");
    assert!(
        err.is_connect() || err.is_timeout(),
        "expected a connect-refused or timeout error, got: {err:?}"
    );
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
        assert_eq!(body_str.as_str(), "{\n    \"hello\": \"index\"\n}");
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
    assert_eq!(body_str.as_str(), "{\n    \"hello\": \"index\"\n}");
}

#[tokio::test]
async fn ipv6_localhost_bound_nonlocalhost_request() {
    let port = ipv6_localhost_listener_setup().await;
    let network_interfaces = list_afinet_netifas().unwrap();
    // Link-local addresses (fe80::/10) are excluded: a bare link-local
    // address is not dialable without a zone/scope ID, so connecting to
    // one fails at the socket layer (EINVAL) before ever reaching the
    // server — never a ConnectionRefused. This is the same case the
    // sibling test `ipv6_global_bound_any_requests` below already skips
    // (there via a string-prefix check; here via the typed API since
    // this is a fresh check, not a tidy-up of that one).
    let ipv6_non_localhost_network_interface = network_interfaces.iter().find(|(_, ip_addr)| {
        matches!(
            ip_addr,
            IpAddr::V6(v6) if !v6.is_loopback() && !v6.is_unicast_link_local()
        )
    });
    match ipv6_non_localhost_network_interface {
        Some((_, ip_addr)) => {
            let host = format!("[{}]", ip_addr);
            // `TestRequest::send()` panics internally on a connection
            // error (it has no fallible variant), so the expected
            // ConnectionRefused is caught via the spawned task's
            // JoinError rather than asserted with #[should_panic] on the
            // whole test — that would also require the "no suitable
            // interface" branch below to panic, which it must not.
            let join_result = tokio::spawn(async move {
                TestRequest::default("/", port)
                    .with_host(host.as_str())
                    .send()
                    .await
            })
            .await;

            match join_result {
                Err(join_err) if join_err.is_panic() => {
                    let payload = join_err.into_panic();
                    let message = payload
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "<non-string panic payload>".to_owned());
                    assert!(
                        message.contains("ConnectionRefused"),
                        "expected a ConnectionRefused panic, got: {message}"
                    );
                }
                Err(join_err) => panic!("task failed unexpectedly: {join_err}"),
                Ok(response) => panic!(
                    "expected connection to a non-localhost address on an \
                     ::1-only listener to be refused, got response status {:?}",
                    response.status()
                ),
            }
        }
        None => {
            // An environment with no globally-routable, non-link-local
            // IPv6 interface (e.g. a typical CI runner) cannot exercise
            // this case at all — there is nothing to connect to that
            // would prove or disprove apimock's binding behaviour.
            // Skipping is correct here, not a workaround: the assertion
            // this test makes has no meaning without such an interface.
            println!(
                "skipping ipv6_localhost_bound_nonlocalhost_request: no \
                 globally-routable, non-link-local IPv6 interface available \
                 in this environment"
            );
        }
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
        assert_eq!(body_str.as_str(), "{\n    \"hello\": \"index\"\n}");
    }
}

/// internal setup fn on ipv4 localhost listener
async fn ipv4_localhost_listener_setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(IPV4_LOCALHOST);
    test_setup.launch().await
}

/// internal setup fn on ipv4 global listener
async fn ipv4_global_listener_setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(IPV4_GLOBAL);
    test_setup.launch().await
}

/// internal setup fn on on ipv6 localhost listener
async fn ipv6_localhost_listener_setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(IPV6_LOCALHOST);
    test_setup.launch().await
}

/// internal setup fn on on ipv6 global listener
async fn ipv6_global_listener_setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(IPV6_GLOBAL);
    test_setup.launch().await
}
