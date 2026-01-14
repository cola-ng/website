use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt::Display;
use std::io;
use std::string::FromUtf8Error;

use async_trait::async_trait;
use salvo::http::{StatusCode, StatusError};
use salvo::oapi::{self, EndpointOutRegister, ToSchema};
use salvo::prelude::{Depot, Request, Response, Writer};
use thiserror::Error;

use crate::DepotExt;
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
    #[error("zip: `{0}`")]
    Zip(#[from] zip::result::ZipError),
    #[error("font parse: `{0}`")]
    FontParse(#[from] ttf_parser::FaceParsingError),
    #[error("http: `{0}`")]
    StatusError(#[from] salvo::http::StatusError),
    #[error("http: `{0}`")]
    StatusInfo(#[from] crate::StatusInfo),
    #[error("login: `{0}`")]
    LoginError(#[from] crate::LoginError),
    #[error("http parse: `{0}`")]
    HttpParse(#[from] salvo::http::ParseError),
    // #[error("pulsar: `{0}`")]
    // Pulsar(#[from] ::pulsar::Error),
    #[error("reqwest: `{0}`")]
    Reqwest(#[from] reqwest::Error),
    #[error("r2d2: `{0}`")]
    R2d2(#[from] diesel::r2d2::PoolError),
    #[error("handlebars render: `{0}`")]
    HandlebarsRender(#[from] handlebars::RenderError),
    #[error("stripe: `{0}`")]
    Stripe(#[from] stripe::StripeError),
    #[error("stripe ParseIdError: `{0}`")]
    ParseIdError(#[from] stripe::ParseIdError),
    #[error("utf8: `{0}`")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("redis: `{0}`")]
    Redis(#[from] redis::RedisError),
    // #[error("consumer: `{0}`")]
    // Consumer(#[from] pulsar::error::ConsumerError),
    #[error("GlobError error: `{0}`")]
    Glob(#[from] globwalk::GlobError),
    #[error("image error: `{0}`")]
    Image(#[from] image::ImageError),
    #[error("PersistError: `{0}`")]
    PersistError(#[from] tempfile::PersistError),
    #[error("S3BuildError: `{0}`")]
    S3BuildError(#[from] aws_sdk_s3::error::BuildError),
    // #[error("NotifyError: `{0}`")]
    // NotifyError(#[from] notify::Error),
    #[error("gemini_rust client error: `{0}`")]
    GeminiClient(#[from] gemini_rust::client::Error),
    #[error("gcp_auth error: `{0}`")]
    GcpAuth(#[from] gcp_auth::Error),
    #[error("base64 decode error: `{0}`")]
    Base64Decode(#[from] base64::DecodeError),
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

#[async_trait]
impl Writer for AppError {
    async fn write(mut self, _req: &mut Request, depot: &mut Depot, res: &mut Response) {
        let code = match &self {
            AppError::StatusError(e) => e.code,
            AppError::LoginError(e) => StatusCode::from_u16(e.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        res.status_code(code);
        let cuser = depot.current_user().ok();
        if let Some(cuser) = &cuser {
            tracing::error!(error = &*self.to_string(), user_id = ?cuser.id, user_name = %cuser.ident_name, "error happened");
        } else {
            tracing::error!(error = &*self.to_string(), "error happened, user not logged in.");
        }
        let in_kernel = cuser.map(|u| u.in_kernel).unwrap_or(false);
        if let AppError::StatusInfo(e) = self {
            res.render(e);
            return;
        }
        if let AppError::LoginError(e) = self {
            res.render(salvo::writing::Json(e));
            return;
        }
        let data = match self {
            AppError::Salvo(e) => {
                if in_kernel {
                    StatusError::internal_server_error()
                        .brief(format!("Unknown error happened in salvo: {e}"))
                        .cause(e)
                } else {
                    StatusError::internal_server_error().brief("Unknown error happened in salvo.")
                }
            }
            AppError::FrequentlyRequest => StatusError::bad_request(),
            AppError::Public(msg) => StatusError::internal_server_error().brief(msg),
            AppError::Internal(msg) => {
                if in_kernel {
                    StatusError::internal_server_error().brief(msg)
                } else {
                    StatusError::internal_server_error()
                }
            }
            AppError::Diesel(e) => {
                tracing::error!(error = ?e, "diesel db error");
                if let diesel::result::Error::NotFound = e {
                    StatusError::not_found().brief("Resource not found.")
                } else if in_kernel {
                    StatusError::internal_server_error().brief(format!("Database error: {e}")).cause(e)
                } else {
                    StatusError::internal_server_error().brief("Database error.")
                }
            }
            AppError::S3BuildError(e) => {
                tracing::error!(error = ?e, "aws s3 build error");
                if in_kernel {
                    StatusError::internal_server_error().brief(format!("Aws s3 build error: {e}")).cause(e)
                } else {
                    StatusError::internal_server_error().brief("Aws s3 build error")
                }
            }
            AppError::Stripe(e) => StatusError::internal_server_error().brief(e.to_string()),
            AppError::StatusError(e) => e,
            e => {
                if in_kernel {
                    StatusError::internal_server_error()
                        .brief(format!("Unknown error happened: {e}"))
                        .cause(e)
                } else {
                    StatusError::internal_server_error().brief("Unknown error happened.")
                }
            }
        };
        res.render(data);
    }
}
impl EndpointOutRegister for AppError {
    fn register(components: &mut oapi::Components, operation: &mut oapi::Operation) {
        operation.responses.insert(
            StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            oapi::Response::new("Internal server error").add_content("application/json", StatusError::to_schema(components)),
        );
        operation.responses.insert(
            StatusCode::NOT_FOUND.as_str(),
            oapi::Response::new("Not found").add_content("application/json", StatusError::to_schema(components)),
        );
        operation.responses.insert(
            StatusCode::BAD_REQUEST.as_str(),
            oapi::Response::new("Bad request").add_content("application/json", StatusError::to_schema(components)),
        );
    }
}

#[derive(Serialize, ToSchema, Debug)]
pub struct LoginError {
    pub user: Option<User>,
    #[salvo(schema(value_type = u16))]
    pub code: u16,
    pub name: String,
    pub brief: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}
impl LoginError {
    pub fn new<N, S, D>(code: StatusCode, name: N, brief: S, detail: D) -> Self
    where
        N: Into<String>,
        S: Into<String>,
        D: Into<String>,
    {
        Self {
            code: code.as_u16(),
            name: name.into(),
            brief: brief.into(),
            detail: Some(detail.into()),
            user: None,
        }
    }
    pub fn bad_request() -> Self {
        Self {
            code: StatusCode::BAD_REQUEST.as_u16(),
            name: "Bad Request".into(),
            brief: "Bad Request".into(),
            detail: None,
            user: None,
        }
    }
    pub fn internal_server_error() -> Self {
        Self {
            code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            name: "Internal server error".into(),
            brief: "Internal server error".into(),
            detail: None,
            user: None,
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn brief(mut self, brief: impl Into<String>) -> Self {
        self.brief = brief.into();
        self
    }

    pub fn user(mut self, user: User) -> Self {
        self.user = Some(user);
        self
    }
}

impl StdError for LoginError {}
impl Display for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.brief, self.code)
    }
}
