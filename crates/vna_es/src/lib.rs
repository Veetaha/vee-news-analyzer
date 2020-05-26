mod index_version;

use anyhow::{Context, Result};
use elasticsearch::{
    http::response::Response as ElasticsearchResponse,
    indices::{IndicesCreateParts, IndicesDeleteParts, IndicesGetAliasParts},
    snapshot::SnapshotCreateRepositoryParts,
    Elasticsearch, SearchParts,
};
pub use index_version::IndexVersion;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::num::NonZeroU32;
use std::{ops::Deref, path::Path};
use vna_es_utils::{es_types, Success};

#[derive(Debug)]
pub struct WithId<T> {
    pub id: String,
    pub doc: T,
}

/// Main document type which is stored in Elasticsearch
#[derive(Debug, Serialize, Deserialize)]
pub struct Article {
    pub category: String,
    pub headline: String,
    pub authors: String,
    pub link: String,
    pub short_description: String,
    pub date: String,
    pub sentiment_score: f32,
    pub sentiment_polarity: SentimentPolarity,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SentimentPolarity {
    Positive,
    Negative,
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
                    "category": {
                        "type": "keyword",
                        "index": true,
                    },
                    "headline": {
                        "type": "text",
                        "index": true,
                    },
                    "authors": {
                        "type": "text",
                        "index": true,
                    },
                    "link": {
                        "type": "keyword",
                        "index": true,
                    },
                    "short_description": {
                        "type": "text",
                        "index": true,
                    },
                    "date": {
                        "type": "date",
                        "index": true,
                    },
                    "sentiment_score": {
                        "type": "float",
                        "index": false,
                    },
                    "sentiment_polarity": {
                        "type": "keyword",
                        "index": true,
                    }
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

    pub async fn fulltext_search(opts: FulltextSearchOpts<'_>) -> Result<Vec<WithId<Article>>> {
        let query = match opts.field_name {
            None => json!({
                "multi_match": {
                    "query": opts.query.deref(),
                    "fields": [
                        "headline^2",
                        "authors",
                        "short_description",
                    ]
                }
            }),
            Some(field_name) => json!({ "match": { field_name: opts.query.deref() } }),
        };

        let response: es_types::SearchResponse<Article> = opts
            .elastic
            .search(SearchParts::Index(&[Self::INDEX_ALIAS]))
            .body(json!({ "query": { "bool": { "should": [query] } } }))
            .send()
            .await?
            .json()
            .await?;

        Ok(response
            .hits
            .hits
            .into_iter()
            .map(|it| WithId {
                id: it._id,
                doc: it._source,
            })
            .collect())
    }

    pub async fn significant_words(
        opts: SignificantWordsOpts<'_>,
    ) -> Result<es_types::SignificantTextAggr> {
        #[derive(Deserialize)]
        pub struct Aggrs {
            pub keywords: es_types::SignificantTextAggr,
        }

        let response: es_types::AggrsResponse<Aggrs> = opts
            .elastic
            .search(SearchParts::Index(&[Self::INDEX_ALIAS]))
            .body(json!({
                // Don't return the document hits array, only the aggregration info
                "size": 0,
                "query": {
                    "match": { opts.field_name: opts.query.deref() }
                },
                "aggs": {
                    "keywords": {
                        "significant_text": {
                            "field": opts.field_name,
                            "size": opts.max_words,
                        }
                    }
                }
            }))
            .send()
            .await?
            .json()
            .await?;

        Ok(response.aggregations.keywords)
    }

    pub async fn sentiment_stats(opts: StatsOpts<'_>) -> Result<Stats> {
        Self::fetch_stats(opts, "sentiment_polarity").await
    }

    pub async fn category_stats(opts: StatsOpts<'_>) -> Result<Stats> {
        Self::fetch_stats(opts, "category").await
    }

    async fn fetch_stats(opts: StatsOpts<'_>, aggr_field: &str) -> Result<Stats> {
        #[derive(Deserialize)]
        pub struct Aggrs {
            pub aggr: es_types::TermsAggr,
        }

        let query = match opts.query {
            Some(query) => json!({ "match": { opts.field_name: query.deref() } }),
            None => json!({ "match_all": {} }),
        };

        let result: es_types::AggrsResponse<Aggrs> = opts
            .elastic
            .search(SearchParts::Index(&[Self::INDEX_ALIAS]))
            .body(json!({
                // Don't return the document hits array, only the aggregration info
                "size": 0,
                "query": query,
                "aggs": {
                    "aggr": {
                        "terms": {
                            "field": aggr_field,
                        }
                    }
                }
            }))
            .send()
            .await?
            .json()
            .await?;

        let buckets = result.aggregations.aggr.buckets;

        let stats = buckets
            .into_iter()
            .map(|it| (it.key, it.doc_count))
            .collect();

        Ok(Stats(stats))
    }
}

pub struct Stats(pub Vec<(String, u64)>);

pub struct StatsOpts<'a> {
    pub elastic: &'a Elasticsearch,
    pub query: &'a Option<stdx::NonHollowString>,
    pub field_name: &'a str,
}

pub struct SignificantWordsOpts<'a> {
    pub elastic: &'a Elasticsearch,
    pub query: &'a stdx::NonHollowString,
    pub field_name: &'a str,
    pub max_words: u32,
}

pub struct FulltextSearchOpts<'a> {
    pub elastic: &'a Elasticsearch,
    pub query: &'a stdx::NonHollowString,
    pub field_name: Option<&'a str>,
}

pub struct CreateArticlesIndexOpts<'a> {
    pub elastic: &'a Elasticsearch,
    pub version: IndexVersion,
    pub number_of_shards: NonZeroU32,
    pub number_of_replicas: u32,
}

pub mod snapshots {
    use super::*;
    use elasticsearch::snapshot::SnapshotCreateParts;

    pub async fn register_repo(
        elastic: &Elasticsearch,
        fs_path: &Path,
        repo: &stdx::NonHollowString,
    ) -> Result<()> {
        let fs_path = fs_path
            .to_str()
            .context("Snapshot repo path contains invalid UTF8 characters")?;

        elastic
            .snapshot()
            .create_repository(SnapshotCreateRepositoryParts::Repository(repo))
            .body(json!({
                "type": "fs",
                "settings": { "location": fs_path, }
            }))
            .send()
            .await?
            .success()
            .await?;

        Ok(())
    }

    pub async fn take_snapshot(
        elastic: &Elasticsearch,
        repo: &stdx::NonHollowString,
        snapshot: &stdx::NonHollowString,
    ) -> Result<()> {
        elastic
            .snapshot()
            .create(SnapshotCreateParts::RepositorySnapshot(repo, snapshot))
            .wait_for_completion(true)
            .send()
            .await?
            .success()
            .await?;
        Ok(())
    }

    /// Restores elasticsearch from snapshot.
    pub async fn restore_from_snapshot(
        elastic: &Elasticsearch,
        repo: &stdx::NonHollowString,
        snapshot: &stdx::NonHollowString,
        index_name: &Option<stdx::NonHollowString>,
    ) -> Result<()> {
        let snap = elastic.snapshot();
        let restore = snap.restore(
            elasticsearch::snapshot::SnapshotRestoreParts::RepositorySnapshot(repo, snapshot),
        );

        if let Some(index_name) = index_name {
            // comma-separated list of indices
            restore
                .body(json!({ "indices": index_name.deref() }))
                .wait_for_completion(true)
                .send()
                .await?
                .success()
                .await?;
        } else {
            restore
                .wait_for_completion(true)
                .send()
                .await?
                .success()
                .await?;
        }

        Ok(())
    }
}
