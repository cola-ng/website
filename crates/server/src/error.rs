use std::borrow::Cow;
use std::io;
use std::string::FromUtf8Error;

use salvo::async_trait;
use salvo::http::{StatusCode, StatusError};
use salvo::oapi::{self, EndpointOutRegister, ToSchema};
use salvo::prelude::{Depot, Request, Response, Writer};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("public: `{0}`")]
    Public(String),
    #[error("internal: `{0}`")]
    Internal(String),
    #[error("salvo internal error: `{0}`")]
    Salvo(#[from] ::salvo::Error),
    #[error("frequently request resource")]
    FrequentlyRequest,
    #[error("io: `{0}`")]
    Io(#[from] io::Error),
    #[error("utf8: `{0}`")]
    FromUtf8(#[from] FromUtf8Error),
    #[error("decoding: `{0}`")]
    Decoding(Cow<'static, str>),
    #[error("url parse: `{0}`")]
    UrlParse(#[from] url::ParseError),
    #[error("serde json: `{0}`")]
    SerdeJson(#[from] serde_json::error::Error),
    #[error("diesel: `{0}`")]
    Diesel(#[from] diesel::result::Error),
    #[error("horme: `{0}`")]
    Horme(#[from] horme::Error),
    #[error("pool: `{0}`")]
    Pool(#[from] crate::db::PoolError),
    #[error("http: `{0}`")]
    StatusError(#[from] salvo::http::StatusError),
    #[error("http parse: `{0}`")]
    HttpParse(#[from] salvo::http::ParseError),
    #[error("utf8: `{0}`")]
    Utf8Error(#[from] std::str::Utf8Error),
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Internal(s.to_string())
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Internal(s)
    }
}

impl From<std::env::VarError> for AppError {
    fn from(e: std::env::VarError) -> Self {
        AppError::Internal(format!("Environment variable error: {}", e))
    }
}

impl AppError {
    pub fn public<S: Into<String>>(msg: S) -> Self {
        Self::Public(msg.into())
    }

    pub fn internal<S: Into<String>>(msg: S) -> Self {
        Self::Internal(msg.into())
    }

    pub fn is_not_found(&self) -> bool {
        match self {
            Self::Diesel(diesel::result::Error::NotFound)
            | Self::Horme(horme::Error::Query(diesel::result::Error::NotFound)) => true,
            _ => false,
        }
    }
}
#[async_trait]
impl Writer for AppError {
    async fn write(mut self, req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        let code = match &self {
            AppError::StatusError(e) => e.code,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        res.status_code(code);
        tracing::error!(
            error = &*self.to_string(),
            "error happened, user not logged in."
        );
        let data = match self {
            AppError::Salvo(_e) => {
                StatusError::internal_server_error().brief("Unknown error happened in salvo.")
            }
            AppError::FrequentlyRequest => StatusError::bad_request(),
            AppError::Public(msg) => StatusError::internal_server_error().brief(msg),
            AppError::Internal(_msg) => StatusError::internal_server_error(),
            AppError::Diesel(e) | AppError::Horme(horme::Error::Query(e)) => {
                tracing::error!(error = ?e, "diesel db error");
                if let diesel::result::Error::NotFound = e {
                    StatusError::not_found().brief("Resource not found.")
                } else {
                    StatusError::internal_server_error().brief("Database error.")
                }
            }
            AppError::StatusError(e) => e,
            _e => StatusError::internal_server_error().brief("Unknown error happened."),
        };
        res.status_code(data.code);
        if req.uri().path().starts_with("/api/") {
            // API clients (including an SSE request rejected before streaming)
            // need a readable JSON error regardless of content negotiation.
            res.render(salvo::prelude::Json(serde_json::json!({
                "code": data.code.as_u16(), "name": data.name,
                "brief": data.brief, "detail": data.detail.unwrap_or_default(),
            })));
        } else {
            res.render(data);
        }
    }
}
impl EndpointOutRegister for AppError {
    fn register(components: &mut oapi::Components, operation: &mut oapi::Operation) {
        operation.responses.insert(
            StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            oapi::Response::new("Internal server error")
                .add_content("application/json", StatusError::to_schema(components)),
        );
        operation.responses.insert(
            StatusCode::NOT_FOUND.as_str(),
            oapi::Response::new("Not found")
                .add_content("application/json", StatusError::to_schema(components)),
        );
        operation.responses.insert(
            StatusCode::BAD_REQUEST.as_str(),
            oapi::Response::new("Bad request")
                .add_content("application/json", StatusError::to_schema(components)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn api_errors_preserve_status_and_json_for_stream_clients() {
        for (error, expected) in [
            (
                AppError::Diesel(diesel::result::Error::NotFound),
                StatusCode::NOT_FOUND,
            ),
            (
                AppError::Horme(horme::Error::Query(diesel::result::Error::NotFound)),
                StatusCode::NOT_FOUND,
            ),
            (AppError::FrequentlyRequest, StatusCode::BAD_REQUEST),
            (
                AppError::StatusError(
                    StatusError::payment_required().brief("Configure your own provider"),
                ),
                StatusCode::PAYMENT_REQUIRED,
            ),
        ] {
            let mut req = Request::new();
            *req.uri_mut() = "/api/qa/errors".parse().unwrap();
            req.headers_mut()
                .insert("Accept", "text/event-stream".parse().unwrap());
            let mut res = Response::new();
            error.write(&mut req, &mut Depot::new(), &mut res).await;
            assert_eq!(res.status_code, Some(expected));
            assert!(
                res.headers()
                    .get("Content-Type")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("application/json")
            );
        }
    }
}
