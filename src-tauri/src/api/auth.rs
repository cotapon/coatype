use reqwest::RequestBuilder;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthKind {
    Bearer,
    ApiKeyHeader { header_name: String },
    None,
}

impl Default for AuthKind {
    fn default() -> Self {
        AuthKind::Bearer
    }
}

pub fn apply_auth(builder: RequestBuilder, kind: &AuthKind, key: &str) -> RequestBuilder {
    match kind {
        AuthKind::Bearer => builder.bearer_auth(key),
        AuthKind::ApiKeyHeader { header_name } => builder.header(header_name.as_str(), key),
        AuthKind::None => builder,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_from_req(req: &reqwest::Request, header: &str) -> Option<String> {
        req.headers()
            .get(header)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    }

    fn build_req(kind: AuthKind, key: &str) -> reqwest::Request {
        let client = reqwest::Client::new();
        let builder = client.get("http://localhost/");
        apply_auth(builder, &kind, key)
            .build()
            .expect("build request")
    }

    #[test]
    fn bearer_sets_authorization_header() {
        let req = build_req(AuthKind::Bearer, "my-secret");
        let val = key_from_req(&req, "authorization").unwrap();
        assert_eq!(val, "Bearer my-secret");
    }

    #[test]
    fn api_key_header_sets_custom_header() {
        let req = build_req(
            AuthKind::ApiKeyHeader {
                header_name: "x-api-key".to_string(),
            },
            "custom-key",
        );
        let val = key_from_req(&req, "x-api-key").unwrap();
        assert_eq!(val, "custom-key");
    }

    #[test]
    fn none_sets_no_auth_header() {
        let req = build_req(AuthKind::None, "ignored");
        assert!(key_from_req(&req, "authorization").is_none());
        assert!(key_from_req(&req, "x-api-key").is_none());
    }
}
