//! web server 启动器程序

use axum::http::StatusCode;
use axum::prelude::*;
use axum::{route, AddExtensionLayer};
use doson::DataValue;
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;

use std::time::Duration;
use tower::timeout::error::Elapsed;
use tower::timeout::TimeoutLayer;
use tower::BoxError;

pub mod db;
pub mod routes;
pub mod secret;
pub mod tools;

pub struct ShareState {
    pub(crate) config: (
        crate::configure::DoreaFileConfig,
        crate::configure::RestConfig,
    ),
    pub(crate) client_addr: (&'static str, u16),
}

pub async fn startup(addr: (&'static str, u16), document_path: &PathBuf) -> crate::Result<()> {
    let hostname = addr.0;
    let dorea_port = addr.1;

    // 读取 rest-service path
    let rest_config = crate::configure::load_rest_config(&document_path)?;

    if !rest_config.switch {
        return Ok(());
    }

    // 全局共享状态数据
    let share_state = Arc::new(ShareState {
        config: (
            crate::configure::load_config(&document_path).unwrap(),
            rest_config.clone(),
        ),
        client_addr: (hostname, dorea_port),
    });

    let rest_port = rest_config.port;
    tokio::task::spawn(async move {

        // 测试数据库连接，并初始化必须数据：
        match crate::client::DoreaClient::connect(
            (hostname, dorea_port),
            &share_state.config.0.connection.connection_password,
        )
        .await
        {
            Ok(mut c) => {
                init_service_system_db(
                    &mut c,
                ).await.unwrap();
            }
            Err(err) => {
                panic!("{}", err);
            }
        };

        let app = route("/", get(routes::index).post(routes::index))
            .route("/auth", post(routes::auth))
            .route("/ping", post(routes::ping))
            .route("/:group/:operation", post(routes::controller))
            .layer(AddExtensionLayer::new(share_state))
            .layer(TimeoutLayer::new(Duration::from_secs(30)));

        let error_handle_app = app.handle_error(|error: BoxError| {
            // Check if the actual error type is `Elapsed` which
            // `Timeout` returns
            if error.is::<Elapsed>() {
                return Ok::<_, Infallible>((
                    StatusCode::REQUEST_TIMEOUT,
                    "Request took too long".into(),
                ));
            }

            // If we encounter some error we don't handle return a generic
            // error
            // Err(error)
            return Ok::<_, Infallible>((
                StatusCode::INTERNAL_SERVER_ERROR,
                // `Cow` lets us return either `&str` or `String`
                Cow::from(format!("Unhandled internal error: {}", error)),
            ));
        });

        let addr = format!("{}:{}", hostname, rest_port);

        log::info!("⍹ >> Web-Service Running at: http://{}/", addr);

        hyper::Server::bind(&addr.parse().unwrap())
            .serve(error_handle_app.into_make_service())
            .await
            .unwrap();
    });

    Ok(())
}

// this function will init the system data.
pub async fn init_service_system_db(client: &mut crate::client::DoreaClient) -> crate::Result<()> {
    client.select("system").await?;

    if client.get("service@accounts").await.is_none() {
        client.setex("service@accounts", DataValue::Dict(HashMap::new()), 0).await?;
    }

    if client.get("service@acc-checker").await.is_none() {
        client.setex("service@acc-checker", DataValue::String(crate::tool::rand_str()), 0).await?;
    }

    Ok(())
}
