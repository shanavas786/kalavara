macro_rules! redirect {
    ($url:expr) => {
        tiny_http::Response::from_string("")
            .with_status_code(307)
            .with_header(tiny_http::Header::from_str($url).unwrap())
    };
}

macro_rules! resp {
    ($body:expr, $status:expr) => {
        tiny_http::Response::from_string($body).with_status_code($status)
    };

    ($body:expr) => {
        tiny_http::Response::from_string($body)
    };
}
