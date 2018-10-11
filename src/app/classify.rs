use h2;
use http;

use proxy::http::classify;

#[derive(Clone, Debug)]
pub struct Classify;

#[derive(Clone, Debug)]
pub struct ClassifyResponse {
    status: Option<http::StatusCode>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Class {
    Grpc(SuccessOrFailure, u32),
    Http(SuccessOrFailure, http::StatusCode),
    Stream(SuccessOrFailure, String),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum SuccessOrFailure { Success, Failure }

impl classify::Classify for Classify {
    type Class = Class;
    type Error = h2::Error;
    type ClassifyResponse = ClassifyResponse;

    fn classify<B>(&self, _: &http::Request<B>) -> Self::ClassifyResponse {
        ClassifyResponse { status: None }
    }
}

impl classify::ClassifyResponse for ClassifyResponse {
    type Class = Class;
    type Error = h2::Error;

    fn start<B>(&mut self, rsp: &http::Response<B>) -> Option<Self::Class> {
        self.status = Some(rsp.status().clone());
        None
    }

    fn eos(&mut self, trailers: Option<&http::HeaderMap>) -> Self::Class {
        if let Some(ref trailers) = trailers {
            let mut grpc_status = trailers
                .get("grpc-status")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u32>().ok());
            if let Some(grpc_status) = grpc_status.take() {
                return if grpc_status == 0 {
                    Class::Grpc(SuccessOrFailure::Success, grpc_status)
                } else {
                    Class::Grpc(SuccessOrFailure::Failure, grpc_status)
                }
            }
        }

        let status = self.status.take().expect("response closed more than once");
        let result = if status.is_server_error() {
            SuccessOrFailure::Failure
        } else {
            SuccessOrFailure::Success
        };
        Class::Http(result, status)
    }

    fn error(&mut self, err: &h2::Error) -> Self::Class {
        Class::Stream(SuccessOrFailure::Failure, format!("{}", err))
    }
}

