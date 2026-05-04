use tracing::error;
pub mod persistent_subscriptions;

pub(crate) fn resolve_authentication(
    options: &crate::options::CommonOperationOptions,
    settings: &crate::ClientSettings,
) -> Option<crate::Authentication> {
    options.authentication.clone().or_else(|| {
        settings
            .default_authenticated_user()
            .as_ref()
            .map(|c| crate::Authentication::Basic(c.clone()))
    })
}

pub fn http_configure_auth(
    builder: reqwest::RequestBuilder,
    auth_opt: Option<&crate::Authentication>,
) -> reqwest::RequestBuilder {
    match auth_opt {
        Some(crate::Authentication::Basic(creds)) => builder.basic_auth(
            String::from_utf8_lossy(creds.login.as_ref()),
            Some(String::from_utf8_lossy(creds.password.as_ref())),
        ),
        Some(crate::Authentication::Bearer(token)) => {
            builder.bearer_auth(String::from_utf8_lossy(token.as_ref()))
        }
        None => builder,
    }
}

pub async fn http_execute_request(
    builder: reqwest::RequestBuilder,
) -> crate::Result<reqwest::Response> {
    let resp = builder.send().await.map_err(|e| {
        if let Some(status) = e.status() {
            match status {
                http::StatusCode::UNAUTHORIZED => crate::Error::AccessDenied,
                http::StatusCode::NOT_FOUND => crate::Error::ResourceNotFound,
                code if code.is_server_error() => crate::Error::ServerError(e.to_string()),
                code => {
                    error!(
                        "Unexpected error when dealing with HTTP request to the server: Code={:?}, {}",
                        code,
                        e
                    );
                    crate::Error::InternalClientError
                }
            }
        } else {
            error!(
                "Unexpected error when dealing with HTTP request to the server: {}",
                e,
            );

            crate::Error::InternalClientError
        }
    })?;

    if resp.status().is_success() {
        return Ok(resp);
    }

    let code = resp.status();
    let msg = resp.text().await.unwrap_or_else(|_| "".to_string());

    match code {
        http::StatusCode::UNAUTHORIZED => Err(crate::Error::AccessDenied),
        http::StatusCode::NOT_FOUND => Err(crate::Error::ResourceNotFound),
        code if code.is_server_error() => Err(crate::Error::ServerError(format!(
            "unexpected server error, reason: {:?}",
            code.canonical_reason()
        ))),
        code => {
            error!(
                "Unexpected error when dealing with HTTP request to the server: Code={:?}: {}",
                code, msg,
            );
            Err(crate::Error::InternalClientError)
        }
    }
}

#[cfg(test)]
mod auth_tests {
    use super::*;
    use crate::options::CommonOperationOptions;
    use crate::{Authentication, ClientSettings, Credentials};

    fn settings_from(connection_string: &str) -> ClientSettings {
        connection_string
            .parse::<ClientSettings>()
            .expect("valid connection string")
    }

    fn authorization_header(builder: reqwest::RequestBuilder) -> Option<String> {
        let request = builder.build().expect("buildable request");
        request
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .map(|v| v.to_str().expect("ASCII header").to_owned())
    }

    fn fresh_builder() -> reqwest::RequestBuilder {
        reqwest::Client::new().get("http://localhost/")
    }

    #[test]
    fn http_configure_auth_with_basic_sets_basic_authorization_header() {
        let auth = Authentication::basic("admin", "changeit");
        let header = authorization_header(http_configure_auth(fresh_builder(), Some(&auth)))
            .expect("authorization header present");
        assert_eq!(header, "Basic YWRtaW46Y2hhbmdlaXQ=");
    }

    #[test]
    fn http_configure_auth_with_bearer_sets_bearer_authorization_header() {
        let auth = Authentication::bearer("abc.def.ghi");
        let header = authorization_header(http_configure_auth(fresh_builder(), Some(&auth)))
            .expect("authorization header present");
        assert_eq!(header, "Bearer abc.def.ghi");
    }

    #[test]
    fn http_configure_auth_with_none_leaves_authorization_unset() {
        assert!(authorization_header(http_configure_auth(fresh_builder(), None)).is_none());
    }

    #[test]
    fn resolve_authentication_prefers_per_call_over_default_user() {
        let settings = settings_from("esdb://admin:changeit@localhost:2113?tls=false");
        let common = CommonOperationOptions {
            authentication: Some(Authentication::bearer("call-token")),
            ..Default::default()
        };

        let resolved = resolve_authentication(&common, &settings).expect("present");
        assert_eq!(resolved, Authentication::bearer("call-token"));
    }

    #[test]
    fn resolve_authentication_falls_back_to_default_user_as_basic() {
        let settings = settings_from("esdb://admin:changeit@localhost:2113?tls=false");
        let common = CommonOperationOptions::default();

        let resolved = resolve_authentication(&common, &settings).expect("present");
        assert_eq!(
            resolved,
            Authentication::Basic(Credentials::new("admin", "changeit"))
        );
    }

    #[test]
    fn resolve_authentication_returns_none_when_neither_configured() {
        let settings = settings_from("esdb://localhost:2113?tls=false");
        let common = CommonOperationOptions::default();

        assert!(resolve_authentication(&common, &settings).is_none());
    }
}
