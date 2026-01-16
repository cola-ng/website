use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt::Display;
use std::io;
use std::string::FromUtf8Error;

use salvo::async_trait;
use salvo::http::{StatusCode, StatusError};
use salvo::oapi::{self, EndpointOutRegister, ToSchema};
use salvo::prelude::{Depot, Request, Response, Writer};
use serde::Serialize;
use thiserror::Error;

use crate::models::User;

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
    #[error("pool: `{0}`")]
    Pool(#[from] crate::db::PoolError),
    #[error("http: `{0}`")]
    StatusError(#[from] salvo::http::StatusError),
    #[error("http parse: `{0}`")]
    HttpParse(#[from] salvo::http::ParseError),
    #[error("r2d2: `{0}`")]
    R2d2(#[from] diesel::r2d2::PoolError),
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
            Self::Diesel(diesel::result::Error::NotFound) => true,
            _ => false,
        }
    }
}
#[async_trait]
impl Writer for AppError {
    async fn write(mut self, _req: &mut Request, depot: &mut Depot, res: &mut Response) {
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
            AppError::Salvo(e) => {
                StatusError::internal_server_error().brief("Unknown error happened in salvo.")
            }
            AppError::FrequentlyRequest => StatusError::bad_request(),
            AppError::Public(msg) => StatusError::internal_server_error().brief(msg),
            AppError::Internal(msg) => StatusError::internal_server_error(),
            AppError::Diesel(e) => {
                tracing::error!(error = ?e, "diesel db error");
                if let diesel::result::Error::NotFound = e {
                    StatusError::not_found().brief("Resource not found.")
                } else {
                    StatusError::internal_server_error().brief("Database error.")
                }
            }
            AppError::StatusError(e) => e,
            e => StatusError::internal_server_error().brief("Unknown error happened."),
        };
        res.render(data);
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
