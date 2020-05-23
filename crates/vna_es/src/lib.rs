mod index_version;

use anyhow::{Context, Result};
use elasticsearch::{
    http::response::Response as ElasticsearchResponse,
    indices::{IndicesCreateParts, IndicesDeleteParts, IndicesGetAliasParts},
    Elasticsearch,
};
pub use index_version::IndexVersion;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::num::NonZeroU32;
use vna_es_utils::{es_types, Success};

pub struct WithId<T> {
    pub id: String,
    pub doc: T,
}

/// Main document type which is stored in Elasticsearch
#[derive(Serialize, Deserialize)]
pub struct Article {
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
        json!({
            "settings": {
                "index": {
                    "number_of_shards": opts.number_of_shards,
                    "number_of_replicas": opts.number_of_replicas,
                }
            },
            "mappings": {
                "properties": {
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
        })
    }

    /// Crates articles index in elasticsearch.
    pub async fn create_index(opts: &CreateArticlesIndexOpts<'_>) -> Result<()> {
        let index_name = opts.version.attach_to_alias(Article::INDEX_ALIAS);
        opts.elastic
            .indices()
            .create(IndicesCreateParts::Index(&index_name))
            .body(Article::index_definition(opts))
            .wait_for_active_shards("all")
            .send()
            .await
            .with_context(|| format!("Failed to create index '{}'", index_name))?
            .success()
            .await?;

        Ok(())
    }

    pub async fn delete_index(elastic: &Elasticsearch, version: IndexVersion) -> Result<()> {
        elastic
            .indices()
            .delete(IndicesDeleteParts::Index(&[
                &version.attach_to_alias(Self::INDEX_ALIAS)
            ]))
            .send()
            .await?
            .success()
            .await?;
        Ok(())
    }

    pub async fn update_index_alias(
        elastic: &Elasticsearch,
        prev_version: Option<IndexVersion>,
        new_version: IndexVersion,
    ) -> Result<()> {
        let mut actions = vec![];

        if let Some(prev_version) = prev_version {
            actions.push(json!({
                "remove": {
                    "index": prev_version.attach_to_alias(Self::INDEX_ALIAS),
                    "alias": Self::INDEX_ALIAS,
                }
            }));
        }

        actions.push(json!({
            "add": {
                "index": new_version.attach_to_alias(Self::INDEX_ALIAS),
                "alias": Self::INDEX_ALIAS,
            }
        }));

        elastic
            .indices()
            .update_aliases()
            .body(json!({ "actions": actions }))
            .send()
            .await?
            .success()
            .await?;

        Ok(())
    }

    pub async fn fetch_index_version(elastic: &Elasticsearch) -> Result<Option<IndexVersion>> {
        let response: ElasticsearchResponse = elastic
            .indices()
            .get_alias(IndicesGetAliasParts::Name(&[Self::INDEX_ALIAS]))
            .send()
            .await?;

        if response.status_code() == http::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        response.error_for_status_code_ref()?;

        let response = response.json::<es_types::GetAliasesResponse>().await?;

        let (index_name,) = response
            .0
            .into_iter()
            .map(|(index_name, _aliases)| index_name)
            .collect_tuple()
            .unwrap_or_else(|| {
                panic!("expected only one index with alias '{}'", Self::INDEX_ALIAS);
            });

        Ok(Some(
            IndexVersion::from_index_name(&index_name)
                .unwrap_or_else(|| panic!("Invalid index name: {}", &index_name)),
        ))
    }
}

pub struct CreateArticlesIndexOpts<'a> {
    pub elastic: &'a Elasticsearch,
    pub version: IndexVersion,
    pub number_of_shards: NonZeroU32,
    pub number_of_replicas: u32,
}
