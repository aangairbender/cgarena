use axum::http::header;
use axum::{
    http::{HeaderValue, StatusCode},
    response::IntoResponse,
};
use bytes::{BufMut, BytesMut};
use serde::Serialize;

use crate::server::errors::Error;

pub type CustomResponseResult<T> = Result<CustomResponse<T>, Error>;

#[derive(Debug)]
pub struct CustomResponse<T: Serialize> {
    pub status_code: StatusCode,
    pub body: Option<T>,
}

impl<T: Serialize> Default for CustomResponse<T> {
    fn default() -> Self {
        Self {
            status_code: StatusCode::OK,
            body: None,
        }
    }
}

impl<T: Serialize> CustomResponse<T> {
    pub fn status_code(mut self, status_code: StatusCode) -> Self {
        self.status_code = status_code;
        self
    }

    pub fn body(mut self, body: T) -> Self {
        self.body = Some(body);
        self
    }
}

impl<T: Serialize> IntoResponse for CustomResponse<T> {
    fn into_response(self) -> axum::response::Response {
        let body = match self.body {
            Some(body) => body,
            None => return (self.status_code).into_response(),
        };

        let mut bytes = BytesMut::new().writer();
        if let Err(err) = serde_json::to_writer(&mut bytes, &body) {
            tracing::error!("Error serializing response body as JSON: {:?}", err);
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }

        let bytes = bytes.into_inner().freeze();
        let headers = [(
            header::CONTENT_TYPE,
            HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
        )];

        (self.status_code, headers, bytes).into_response()
    }
}
