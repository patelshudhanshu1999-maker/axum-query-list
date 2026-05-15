use axum_core::extract::FromRequestParts;
use http::request::Parts;
use serde::de::DeserializeOwned;

fn deserialize_pair<T: DeserializeOwned>(
    key: &str,
    value: &str,
) -> Result<T, serde_json::Error> {
    if let Ok(n) = value.parse::<i64>() {
        serde_json::from_value(serde_json::json!({ key: n }))
    } else if let Ok(b) = value.parse::<bool>() {
        serde_json::from_value(serde_json::json!({ key: b }))
    } else {
        serde_json::from_value(serde_json::json!({ key: value }))
    }
}

fn parse_and_cap<T: DeserializeOwned, const MAX: usize>(
    query: &str,
) -> Result<Vec<T>, QueryListError> {
    let items: Vec<T> = form_urlencoded::parse(query.as_bytes())
        .map(|(k, v)| {
            deserialize_pair::<T>(k.as_ref(), v.as_ref())
                .map_err(|e| QueryListError(e.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if items.len() > MAX {
        return Err(QueryListError(format!("too many query parameters, max allowed is {MAX}")));
    }
    Ok(items)
}

/// Error type for QueryList
#[derive(Debug, thiserror::Error)]
#[error("Failed to deserialize query string: {0}")]
pub struct QueryListError(String);

impl axum_core::response::IntoResponse for QueryListError {
    fn into_response(self) -> axum_core::response::Response {
        (http::StatusCode::BAD_REQUEST, self.0).into_response()
    }
}

/// Extractor that deserializes query string into `Vec<T>`.
/// Each key-value pair is deserialized into one `T`.
///
/// `MAX` limits the total number of items. Defaults to unlimited.
#[derive(Debug, Clone, Default)]
pub struct QueryList<T, const MAX: usize = { usize::MAX }>(pub Vec<T>);

impl<T, S, const MAX: usize> FromRequestParts<S> for QueryList<T, MAX>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = QueryListError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().ok_or_else(|| QueryListError("missing query string".to_string()))?;
        Ok(Self(parse_and_cap::<T, MAX>(query)?))
    }
}

/// Extractor that deserializes query string into `Vec<T>`.
/// Returns empty `Vec` if no query parameters are present.
///
/// `MAX` limits the total number of items. Defaults to unlimited.
#[derive(Debug, Clone, Default)]
pub struct OptionalQueryList<T, const MAX: usize = { usize::MAX }>(pub Vec<T>);

impl<T, S, const MAX: usize> FromRequestParts<S> for OptionalQueryList<T, MAX>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = QueryListError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or_default();
        Ok(Self(parse_and_cap::<T, MAX>(query)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    #[allow(dead_code)]
    enum List {
        Id(u32),
        Username(String),
    }

    fn make_parts(uri: &str) -> Parts {
        http::Request::builder()
            .uri(uri.parse::<http::Uri>().unwrap())
            .body(())
            .unwrap()
            .into_parts()
            .0
    }

    #[tokio::test]
    async fn test_query_list_enum() {
        let mut parts = make_parts("/?id=123&username=abc&id=345");
        let result = QueryList::<List>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.len(), 3);
    }

    #[tokio::test]
    async fn test_cap_limit() {
        let mut parts = make_parts("/?id=1&id=2&id=3");
        let result = QueryList::<List, 2>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_optional_query_list_no_query_string() {
        let mut parts = make_parts("/");
        let result = OptionalQueryList::<List>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.len(), 0);
    }

    #[tokio::test]
    async fn test_query_list_requires_query_string() {
        let mut parts = make_parts("/");
        let result = QueryList::<List>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_percent_encoded_value() {
        let mut parts = make_parts("/?username=hello%20world");
        let result = QueryList::<List>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0[0], List::Username("hello world".to_string()));
    }

    #[tokio::test]
    async fn test_plus_as_space_in_value() {
        let mut parts = make_parts("/?username=hello+world");
        let result = QueryList::<List>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0[0], List::Username("hello world".to_string()));
    }

    #[tokio::test]
    async fn test_unknown_variant_is_rejected() {
        let mut parts = make_parts("/?unknownKey=42");
        let result = QueryList::<List>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_optional_query_list_with_params() {
        let mut parts = make_parts("/?id=7&id=8");
        let result = OptionalQueryList::<List>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.len(), 2);
    }

    #[tokio::test]
    async fn test_optional_query_list_cap_limit() {
        let mut parts = make_parts("/?id=1&id=2&id=3");
        let result = OptionalQueryList::<List, 2>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_percent_encoded_key() {
        let mut parts = make_parts("/?i%64=99");
        let result = QueryList::<List>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0[0], List::Id(99));
    }
}
