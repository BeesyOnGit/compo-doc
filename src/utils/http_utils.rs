use axum::{
    body::Body,
    http::{Response, StatusCode},
};
use serde::Serialize;

pub fn json_response_builder<T: Serialize>(status: StatusCode, value: T) -> Response<Body> {
    let json_body = serde_json::to_string(&value).unwrap();
    return Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(json_body))
        .unwrap();
}
