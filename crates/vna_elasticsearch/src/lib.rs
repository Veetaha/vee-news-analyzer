use anyhow::{Context, Result};
use elasticsearch::{
    http::transport::{SingleNodeConnectionPool, TransportBuilder},
    indices::IndicesCreateParts,
    Elasticsearch,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::num::NonZeroU32;
use url::Url;

pub struct WithId<T> {
    pub id: String,
    pub doc: T,
}

/// Main document type which is stored in Elasticsearch
#[derive(Serialize, Deserialize)]
pub struct Article {
    pub source_id: String,
    pub source_name: String,
    pub author: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub url_to_image: String,
    pub published_at: String,
    pub content: String,
}

impl Article {
    pub const INDEX_ALIAS: &'static str = "articles";

    /// Creates Elasticsearch article documents mapping declaration object
    fn index_definition(opts: &CreateArticlesIndexOpts<'_>) -> impl Serialize {
        json! {{
            "settings": {
                "index": {
                    "number_of_shards": opts.number_of_shards,
                    "number_of_replicas": opts.number_of_replicas,
                }
            },
            "mappings": {
                "properties": {
                    "source_id": {
                        "type": "keyword",
                        "index": true,
                    },
                    "source_name": {
                        "type": "text",
                        "index": true,
                    },
                    "author": {
                        "type": "text",
                        "index": true,
                    },
                    "title": {
                        "type": "text",
                        "index": true,
                    },
                    "description": {
                        "type": "text",
                        "index": true,
                    },
                    "url": {
                        "type": "keyword",
                        "index": true,
                    },
                    "url_to_image": {
                        "type": "keyword",
                        "index": true,
                    },
                    "published_at": {
                        "type": "date",
                        "index": true,
                    },
                    "content": {
                        "type": "text",
                        "index": true,
                    },
                }
            },
        }}
    }
}

pub struct CreateArticlesIndexOpts<'a> {
    pub elastic: &'a Elasticsearch,
    pub version: u32,
    pub number_of_shards: NonZeroU32,
    pub number_of_replicas: u32,
}

/// Crates articles index in elasticsearch.
pub async fn create_articles_index(opts: &CreateArticlesIndexOpts<'_>) -> Result<()> {
    let index_name = versioned_index_name(Article::INDEX_ALIAS, opts.version);
    opts.elastic
        .indices()
        .create(IndicesCreateParts::Index(&index_name))
        .body(Article::index_definition(opts))
        .wait_for_active_shards("all")
        .send()
        .await
        .with_context(|| format!("Failed to create index '{}'", index_name))?
        .error_for_status_code()?;

    Ok(())
}

/// Create an instance of simple proxy-less elasticsearch client (not the one on Elastic cloud)
pub fn create_elasticsearch_client(url: Url) -> Result<Elasticsearch> {
    let conn_pool = SingleNodeConnectionPool::new(url);
    let transport = TransportBuilder::new(conn_pool).disable_proxy().build()?;
    Ok(Elasticsearch::new(transport))
}

pub fn versioned_index_name(alias: &str, version: u32) -> String {
    format!("{}_v{}", alias, version)
}
