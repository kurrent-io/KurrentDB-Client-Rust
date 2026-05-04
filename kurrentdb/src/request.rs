use crate::options::CommonOperationOptions;
use crate::{Authentication, ClientSettings, Credentials, NodePreference};
use base64::Engine;
use std::borrow::Cow;

pub(crate) fn build_request_metadata(
    settings: &ClientSettings,
    options: &CommonOperationOptions,
) -> tonic::metadata::MetadataMap
where
{
    use tonic::metadata::MetadataValue;

    let mut metadata = tonic::metadata::MetadataMap::new();
    let authentication: Option<Cow<'_, Authentication>> = options
        .authentication
        .as_ref()
        .map(Cow::Borrowed)
        .or_else(|| {
            settings
                .default_authenticated_user()
                .as_ref()
                .map(|c| Cow::Owned(Authentication::Basic(c.clone())))
        });

    if let Some(auth) = authentication.as_deref() {
        if let Some(header_value) = build_authorization_header(auth) {
            metadata.insert("authorization", header_value);
        }
    }

    if options.requires_leader || settings.node_preference() == NodePreference::Leader {
        let header_value = MetadataValue::try_from("true").expect("valid metadata header value");
        metadata.insert("requires-leader", header_value);
    }

    if let Some(conn_name) = settings.connection_name.as_ref() {
        let header_value =
            MetadataValue::try_from(conn_name.as_str()).expect("valid metadata header value");
        metadata.insert("connection-name", header_value);
    }

    metadata
}

fn build_authorization_header(
    auth: &Authentication,
) -> Option<tonic::metadata::MetadataValue<tonic::metadata::Ascii>> {
    use tonic::metadata::MetadataValue;

    let (header, kind) = match auth {
        Authentication::Basic(Credentials { login, password }) => {
            let login = String::from_utf8_lossy(login);
            let password = String::from_utf8_lossy(password);
            let encoded = base64::engine::general_purpose::STANDARD
                .encode(format!("{}:{}", login, password));
            (format!("Basic {}", encoded), "basic")
        }
        Authentication::Bearer(token) => {
            let token = String::from_utf8_lossy(token);
            (format!("Bearer {}", token), "bearer")
        }
    };

    match MetadataValue::try_from(header.as_str()) {
        Ok(value) => Some(value),
        Err(_) => {
            // HTTP/2 header values reject control characters (NUL, LF, CR, others < 0x20
            // except tab, plus DEL). Bearer tokens are sent verbatim, so an untrimmed
            // trailing newline or similar in the token would panic if we used `.expect()`.
            // We skip the header instead; the server will respond with AccessDenied, and
            // this log explains why. The token itself is never logged.
            tracing::warn!(
                auth_kind = kind,
                "authentication value contains characters that are not valid in a gRPC metadata header; the Authorization header will be omitted"
            );
            None
        }
    }
}

#[cfg(test)]
mod auth_tests {
    use super::*;
    use crate::AppendToStreamOptions;
    use crate::options::Options;

    #[test]
    fn basic_authentication_produces_base64_basic_header() {
        let auth = Authentication::basic("admin", "changeit");
        let header = build_authorization_header(&auth).expect("ASCII header");
        // base64("admin:changeit") = YWRtaW46Y2hhbmdlaXQ=
        assert_eq!(header.to_str().unwrap(), "Basic YWRtaW46Y2hhbmdlaXQ=");
    }

    #[test]
    fn bearer_authentication_produces_bearer_header_verbatim() {
        let auth = Authentication::bearer("abc.def.ghi");
        let header = build_authorization_header(&auth).expect("ASCII header");
        assert_eq!(header.to_str().unwrap(), "Bearer abc.def.ghi");
    }

    #[test]
    fn basic_authentication_with_special_chars_encodes_correctly() {
        let auth = Authentication::basic("user@example.com", "p@ss:word");
        let header = build_authorization_header(&auth).expect("ASCII header");
        // base64("user@example.com:p@ss:word") = dXNlckBleGFtcGxlLmNvbTpwQHNzOndvcmQ=
        assert_eq!(
            header.to_str().unwrap(),
            "Basic dXNlckBleGFtcGxlLmNvbTpwQHNzOndvcmQ="
        );
    }

    #[test]
    fn credentials_convert_into_basic_authentication() {
        let auth: Authentication = Credentials::new("admin", "changeit").into();
        let header = build_authorization_header(&auth).expect("ASCII header");
        assert_eq!(header.to_str().unwrap(), "Basic YWRtaW46Y2hhbmdlaXQ=");
    }

    #[test]
    fn bearer_with_embedded_newline_returns_none_instead_of_panicking() {
        // HTTP/2 header values reject embedded LF. A token with a trailing newline is
        // a realistic failure mode (e.g. read from a file without trimming). The function
        // must not panic; it must return None so the caller skips the header.
        let auth = Authentication::bearer("token\nleak");
        assert!(build_authorization_header(&auth).is_none());
    }

    #[test]
    fn bearer_with_embedded_null_returns_none_instead_of_panicking() {
        let auth = Authentication::bearer("token\0bad");
        assert!(build_authorization_header(&auth).is_none());
    }

    #[test]
    fn build_request_metadata_skips_bearer_token_with_invalid_chars() {
        let settings = settings_from("esdb://localhost:2113?tls=false");
        let options = AppendToStreamOptions::default()
            .authenticated(Authentication::bearer("token\nleak"));
        // Must not panic. The authorization header should simply be absent so the server
        // responds with AccessDenied rather than the client crashing.
        let metadata = build_request_metadata(&settings, options.common_operation_options());
        assert!(metadata.get("authorization").is_none());
    }

    fn settings_from(connection_string: &str) -> ClientSettings {
        connection_string
            .parse::<ClientSettings>()
            .expect("valid connection string")
    }

    #[test]
    fn no_auth_anywhere_produces_no_authorization_header() {
        let settings = settings_from("esdb://localhost:2113?tls=false");
        let options = AppendToStreamOptions::default();
        let metadata = build_request_metadata(&settings, options.common_operation_options());

        assert!(metadata.get("authorization").is_none());
    }

    #[test]
    fn default_user_from_connection_string_falls_through_as_basic() {
        let settings = settings_from("esdb://admin:changeit@localhost:2113?tls=false");
        let options = AppendToStreamOptions::default();
        let metadata = build_request_metadata(&settings, options.common_operation_options());

        assert_eq!(
            metadata.get("authorization").unwrap().to_str().unwrap(),
            "Basic YWRtaW46Y2hhbmdlaXQ="
        );
    }

    #[test]
    fn per_call_bearer_overrides_default_user() {
        let settings = settings_from("esdb://admin:changeit@localhost:2113?tls=false");
        let options = AppendToStreamOptions::default()
            .authenticated(Authentication::bearer("call-token"));
        let metadata = build_request_metadata(&settings, options.common_operation_options());

        assert_eq!(
            metadata.get("authorization").unwrap().to_str().unwrap(),
            "Bearer call-token"
        );
    }

    #[test]
    fn authenticated_builder_accepts_credentials_directly() {
        let settings = settings_from("esdb://localhost:2113?tls=false");
        let options =
            AppendToStreamOptions::default().authenticated(Credentials::new("alice", "secret"));
        let metadata = build_request_metadata(&settings, options.common_operation_options());

        // base64("alice:secret") = YWxpY2U6c2VjcmV0
        assert_eq!(
            metadata.get("authorization").unwrap().to_str().unwrap(),
            "Basic YWxpY2U6c2VjcmV0"
        );
    }

    #[test]
    fn authenticated_builder_accepts_authentication_bearer() {
        let settings = settings_from("esdb://localhost:2113?tls=false");
        let options =
            AppendToStreamOptions::default().authenticated(Authentication::bearer("eyJ.payload"));
        let metadata = build_request_metadata(&settings, options.common_operation_options());

        assert_eq!(
            metadata.get("authorization").unwrap().to_str().unwrap(),
            "Bearer eyJ.payload"
        );
    }
}
