use anyhow::Result;
use elasticsearch::{http::request::NdBody, params::Refresh, Elasticsearch};
use newsapi::payload::article::{Article as NewsApiArticle, ArticleSource};
use std::{iter, num::NonZeroU32, time::Duration};
use url::Url;

mod news_api;

pub struct RunOpts {
    pub es_url: Url,
    pub news_api_key: String,
    pub scrape_interval: Option<u32>,
    pub max_news: u64,
    pub n_shards: NonZeroU32,
    pub n_replicas: u32,
}

#[derive(Default)]
pub struct Stats {
    pub total_processed: u64,
    pub total_indexed: u64,
}

pub async fn run(
    RunOpts {
        es_url,
        max_news,
        n_replicas,
        n_shards,
        news_api_key,
        scrape_interval,
    }: RunOpts,
) -> Result<()> {
    let elastic = &vna_elasticsearch::create_elasticsearch_client(es_url)?;

    let scrape = || async {
        let _t = stdx::debug_time_it("Scraping news api");

        vna_elasticsearch::create_articles_index(&vna_elasticsearch::CreateArticlesIndexOpts {
            elastic,
            version: 1,
            number_of_replicas: n_replicas,
            number_of_shards: n_shards,
        })
        .await?;

        let mut stats = Stats::default();

        for batch in news_api::ArticlesPagination::new(&news_api_key) {
            stats.total_processed += batch.articles.len() as u64;

            let mut bulk_body: Vec<_> = batch
                .articles
                .into_iter()
                .filter_map(article_doc_from_news_api)
                .map(|doc| {
                    let header = "{\"index\":{}}".to_owned();
                    let doc = serde_json::to_string(&doc).unwrap();
                    iter::once(header).chain(iter::once(doc))
                })
                .flatten()
                .collect();

            stats.total_indexed += bulk_body.len() as u64; // FIXME: be more pessimistic (check response)

            elastic
                .bulk(elasticsearch::BulkParts::Index(
                    vna_elasticsearch::Article::INDEX_ALIAS,
                ))
                .body(bulk_body)
                .refresh(Refresh::WaitFor)
                .wait_for_active_shards("all")
                .send()
                .await?;

            if stats.total_indexed >= max_news {
                break;
            }
        }

        // TODO: set the alias to new index, afterwards delete the old index

        Result::<()>::Ok(())
    };

    let scrape_interval = match &scrape_interval {
        Some(it) => *it,
        None => {
            scrape().await?;
            return Ok(());
        }
    };
    let scrape_interval = Duration::from_millis(scrape_interval as u64);

    loop {
        scrape().await?;
        std::thread::sleep(scrape_interval);
    }
}

fn article_doc_from_news_api(article: NewsApiArticle) -> Option<vna_elasticsearch::Article> {
    match article {
        NewsApiArticle {
            source:
                ArticleSource {
                    id: Some(source_id),
                    name: source_name,
                },
            author: Some(author),
            title,
            description: Some(description),
            url,
            url_to_image: Some(url_to_image),
            published_at,
            content: Some(content),
        } => {
            if !has_meaningful_text_content(&content) {
                return None;
            }
            Some(vna_elasticsearch::Article {
                author,
                title,
                source_name,
                source_id,
                content,
                description,
                published_at,
                url_to_image,
                url,
            })
        }
        _ => None,
    }
}

fn has_meaningful_text_content(suspect: &str) -> bool {
    suspect.match_indices('\n').count() >= 2
}
