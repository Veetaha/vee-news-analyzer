use anyhow::Result;
use elasticsearch::params::Refresh;
use newsapi::payload::article::{Article as NewsApiArticle, ArticleSource};
use std::{iter, num::NonZeroU32, time::Duration};
use url::Url;
use vna_es; // FIMXE: rename vna_es -> vna_es to shorten the code

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
    pub new_index_name: String,
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
) -> Result<Stats> {
    let elastic = &vna_es_utils::create_elasticsearch_client(es_url)?;

    let scrape = || async {
        let _t = stdx::debug_time_it("Scraping news api");

        let prev_index_version = vna_es::Article::fetch_index_version(elastic).await?;
        let new_index_version = prev_index_version
            .map(vna_es::IndexVersion::incremented)
            .unwrap_or_default();

        vna_es::Article::create_index(&vna_es::CreateArticlesIndexOpts {
            elastic,
            version: new_index_version,
            number_of_replicas: n_replicas,
            number_of_shards: n_shards,
        })
        .await?;

        let mut stats = Stats {
            total_indexed: 0,
            total_processed: 0,
            new_index_name: new_index_version.attach_to_alias(vna_es::Article::INDEX_ALIAS),
        };
        let mut article_pages = news_api::ArticlesPagination::new(&news_api_key);

        while let Some(page) = article_pages.next().await {
            stats.total_processed += page.articles.len() as u64;

            let bulk_body: Vec<_> = page
                .articles
                .into_iter()
                .filter_map(article_doc_from_news_api)
                .take((max_news - stats.total_indexed) as usize)
                .map(|doc| {
                    let header = "{\"index\":{}}".to_owned();
                    let doc = serde_json::to_string(&doc).unwrap();
                    iter::once(header).chain(iter::once(doc))
                })
                .flatten()
                .collect();

            let n_bulk_docs = bulk_body.len() / 2;

            log::debug!("Ingesting {}", n_bulk_docs);

            stats.total_indexed += n_bulk_docs as u64; // FIXME: be more pessimistic (check response)

            if bulk_body.len() != 0 {
                elastic
                    .bulk(elasticsearch::BulkParts::Index(&stats.new_index_name))
                    .body(bulk_body)
                    .refresh(Refresh::WaitFor)
                    .send()
                    .await?;
            }

            if stats.total_indexed >= max_news {
                break;
            }
        }

        vna_es::Article::update_index_alias(elastic, prev_index_version, new_index_version).await?;
        Result::<_>::Ok(stats)
    };

    let scrape_interval = match &scrape_interval {
        Some(it) => *it,
        None => {
            return scrape().await;
        }
    };
    let scrape_interval = Duration::from_millis(scrape_interval as u64);

    loop {
        scrape().await?;
        std::thread::sleep(scrape_interval);
    }
}

fn article_doc_from_news_api(article: NewsApiArticle) -> Option<vna_es::Article> {
    match article {
        NewsApiArticle {
            source:
                ArticleSource {
                    id: _,
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
            Some(vna_es::Article {
                author,
                title,
                source_name,
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

fn has_meaningful_text_content(_suspect: &str) -> bool {
    true
    // FIXME:
    // suspect.match_indices('\n').count() >= 2
}
