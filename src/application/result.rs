use crate::errors::BankError;
use crate::model::BankAccountError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use std::borrow::Cow;

pub type HttpResult = Result<Response, BankError>;

impl IntoResponse for BankError {
    fn into_response(self) -> Response {
        let http_error = HttpError::from_error(self.into());
        http_error.into_response()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorReport {
    pub error: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub backtrace: Option<String>,
}

impl From<anyhow::Error> for ErrorReport {
    fn from(error: anyhow::Error) -> Self {
        Self {
            error: error.to_string(),
            error_code: None,
            backtrace: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum HttpError {
    BadRequest { error: ErrorReport },
    NotFound { message: Cow<'static, str> },
    Internal { error: ErrorReport },
}

impl HttpError {
    fn from_error(error: anyhow::Error) -> Self {
        tracing::error!("HTTP handler error: {error}");
        match error.downcast_ref::<BankError>() {
            Some(BankError::BankAccount(BankAccountError::NotFound(account_id))) => {
                Self::NotFound {
                    message: format!("No bank account found for account id: {account_id}").into(),
                }
            },
            Some(BankError::BankAccount(_)) => Self::BadRequest { error: error.into() },
            Some(BankError::Api(_)) => Self::Internal { error: error.into() },
            Some(BankError::User(_)) => Self::BadRequest { error: error.into() },
            Some(_) => Self::Internal { error: error.into() },
            None => Self::Internal { error: error.into() },
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound { message } => (StatusCode::NOT_FOUND, Json(message)).into_response(),
            Self::BadRequest { error } => (StatusCode::BAD_REQUEST, Json(error)).into_response(),
            Self::Internal { error } => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
            },
        }
    }
}

// #[derive(Debug, Clone)]
// #[repr(transparent)]
// pub struct ServiceError(anyhow::Error);
//
// impl<E: std::error::Error> IntoResponse for ServiceError {
//     fn into_response(self) -> Response {
//         HttpError::from_error(self.into()).into_response()
//     }
// }
//
// impl<E: std::error::Error> Into<ServiceError> for E {
//     #[inline]
//     fn into(self) -> ServiceError {
//         ServiceError(self.into())
//     }
// }

// impl<T: IntoResponse> Into<HttpResult> for T {
//     fn into(self) -> HttpResult {
//         Ok(self.into_response())
//     }
// }
// pub trait IntoHttp {
//     fn into_http(self) -> HttpResult;
// }
//
// impl<T: IntoResponse> IntoHttp for T {
//     fn into_http(self) -> HttpResult {
//         Ok(self.into_response())
//     }
// }
