//! Assorted utilities for elasticsearch

pub use elasticsearch;
pub mod es_types;
use anyhow::{bail, Result};
use elasticsearch::{
    http::{
        response::Response as ElasticsearchResponse,
        transport::{SingleNodeConnectionPool, TransportBuilder},
    },
    Elasticsearch,
};
use std::{future::Future, pin::Pin};
use url::Url;

pub trait Success {
    type Ok;
    type Err;
    fn success(self) -> Pin<Box<dyn Future<Output = Result<Self::Ok, Self::Err>>>>;
}

impl Success for ElasticsearchResponse {
    type Ok = ();
    type Err = anyhow::Error;
    fn success(self) -> Pin<Box<dyn Future<Output = Result<Self::Ok, Self::Err>>>> {
        Box::pin(async move {
            let status_code = self.status_code();
            if !status_code.is_success() {
                let backtrace = backtrace::Backtrace::new();
                let res: serde_json::Value = self.json().await?;
                bail!(
                    "Elasticsearch returned {} response: {:#} at\n{:?}",
                    status_code,
                    res,
                    backtrace
                );
            }
            Ok(())
        })
    }
}

/// Create an instance of simple proxy-less elasticsearch client (not the one on Elastic cloud)
pub fn create_elasticsearch_client(url: Url) -> Result<Elasticsearch> {
    let conn_pool = SingleNodeConnectionPool::new(url);
    let transport = TransportBuilder::new(conn_pool).disable_proxy().build()?;
    Ok(Elasticsearch::new(transport))
}

#[macro_export]
macro_rules! elasticsearch_client_or_return {
    () => {
        match $crate::try_create_elasticsearch_client() {
            Some(it) => it,
            None => return,
        }
    };
}

// FIXME: Conditionally exclde test code
pub fn try_create_elasticsearch_client() -> Option<Elasticsearch> {
    let es_url = Some(std::env::var("VNA_TEST_ES_URL").ok()?.parse().unwrap())?;
    Some(create_elasticsearch_client(es_url).unwrap())
}
