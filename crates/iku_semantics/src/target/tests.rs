use super::*;
use bytes::Bytes;

fn authority(text: &str) -> Authority {
    Authority::new(text.as_bytes()).unwrap()
}

fn path(text: &'static str) -> PathAndQuery {
    PathAndQuery::new(Bytes::from_static(text.as_bytes())).unwrap()
}

#[test]
fn target_uri_accepts_http_with_an_authority() {
    let uri = TargetUri::new(Scheme::HTTP, Some(authority("example.com")), path("/a?b")).unwrap();
    assert_eq!(uri.scheme(), &Scheme::HTTP);
    assert_eq!(uri.authority(), Some(&authority("example.com")));
    assert_eq!(uri.path_and_query(), &path("/a?b"));
}

#[test]
fn target_uri_rejects_http_and_https_without_an_authority() {
    // RFC 9110 §4.2.1, §4.2.2.
    assert_eq!(
        TargetUri::new(Scheme::HTTP, None, PathAndQuery::ROOT),
        Err(MissingAuthorityError)
    );
    assert_eq!(
        TargetUri::new(Scheme::HTTPS, None, PathAndQuery::ROOT),
        Err(MissingAuthorityError)
    );
    // The scheme is lowercased on construction, so HTTP spelled in capitals is the same.
    let upper = Scheme::from_static("HTTP").unwrap();
    assert_eq!(
        TargetUri::new(upper, None, PathAndQuery::ROOT),
        Err(MissingAuthorityError)
    );
}

#[test]
fn target_uri_allows_another_scheme_without_an_authority() {
    let scheme = Scheme::from_static("urn").unwrap();
    let uri = TargetUri::new(scheme.clone(), None, path("/x")).unwrap();
    assert_eq!(uri.scheme(), &scheme);
    assert_eq!(uri.authority(), None);
}

#[test]
fn target_uri_allows_another_scheme_with_an_authority() {
    let scheme = Scheme::from_static("ftp").unwrap();
    assert!(TargetUri::new(scheme, Some(authority("h:21")), PathAndQuery::ROOT).is_ok());
}

#[test]
fn target_uri_into_parts_gives_the_components_back() {
    let uri = TargetUri::new(Scheme::HTTPS, Some(authority("a.example")), path("/p?q")).unwrap();
    let (scheme, auth, pq) = uri.into_parts();
    assert_eq!(scheme, Scheme::HTTPS);
    assert_eq!(auth, Some(authority("a.example")));
    assert_eq!(pq, path("/p?q"));
}

#[test]
fn request_target_variants_compare() {
    let uri = TargetUri::new(Scheme::HTTP, Some(authority("h")), PathAndQuery::ROOT).unwrap();
    let asterisk =
        RequestTarget::Asterisk(AsteriskTarget::new(Scheme::HTTP, Some(authority("h"))).unwrap());
    assert_eq!(asterisk, asterisk.clone());
    assert_ne!(asterisk, RequestTarget::Uri(uri));
    assert_eq!(
        RequestTarget::Authority(authority("h:1")),
        RequestTarget::Authority(authority("H:1"))
    );
}

#[test]
fn missing_authority_error_displays() {
    assert_eq!(
        alloc::format!("{MissingAuthorityError}"),
        "an http or https target URI has no authority"
    );
}

#[test]
fn asterisk_target_holds_the_scheme_and_the_authority() {
    let target = AsteriskTarget::new(Scheme::HTTPS, Some(authority("example.com:8443"))).unwrap();
    assert_eq!(target.scheme(), &Scheme::HTTPS);
    assert_eq!(target.authority(), Some(&authority("example.com:8443")));
    assert_eq!(
        target.into_parts(),
        (Scheme::HTTPS, Some(authority("example.com:8443")))
    );
}

#[test]
fn asterisk_target_rejects_http_and_https_without_an_authority() {
    assert_eq!(
        AsteriskTarget::new(Scheme::HTTP, None),
        Err(MissingAuthorityError)
    );
    assert_eq!(
        AsteriskTarget::new(Scheme::HTTPS, None),
        Err(MissingAuthorityError)
    );
}
