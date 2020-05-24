use anyhow::Result;
use elasticsearch::{params::Refresh, Elasticsearch};
use itertools::Itertools;
use std::{iter, num::NonZeroU32, path::Path, time::Duration};

mod data_source;

pub struct RunOpts<'a> {
    pub elastic: &'a Elasticsearch,
    pub kaggle_dataset_path: &'a Path,
    pub scrape_interval: Option<u32>,
    pub max_news: u64,
    pub n_shards: NonZeroU32,
    pub n_replicas: u32,
    pub ingest_batch: NonZeroU32,
    pub leave_old_index: bool,
}

#[derive(Default)]
pub struct Stats {
    pub total_indexed: u64,
    pub new_index_name: String,
}

pub async fn run(
    RunOpts {
        elastic,
        max_news,
        n_replicas,
        n_shards,
        kaggle_dataset_path,
        scrape_interval,
        ingest_batch,
        leave_old_index,
    }: RunOpts<'_>,
) -> Result<Stats> {
    let n_cpus = num_cpus::get();

    let scrape = || async {
        let _t = stdx::debug_time_it("Scraping datasources");

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
            new_index_name: new_index_version.attach_to_alias(vna_es::Article::INDEX_ALIAS),
        };
        let articles = data_source::kaggle::read_articles(kaggle_dataset_path)?
            .chunks(ingest_batch.get() as usize);

        for batch in &articles {
            let _ti = stdx::debug_time_it("Ingesting a batch");

            let bulk_body: Vec<_> = {
                let _ta = stdx::debug_time_it("Analyzing batch");
                let max_take = (max_news - stats.total_indexed) as usize;

                let per_thread_chunks = batch
                    .take(max_take)
                    .chunks(ingest_batch.get() as usize / n_cpus);

                let tasks = per_thread_chunks.into_iter().map(|per_thread_batch| {
                    let docs: Vec<_> = per_thread_batch.collect();

                    tokio::task::spawn_blocking(move || {
                        docs.into_iter()
                            .map(kaggle_article_to_es_document)
                            .collect::<Vec<_>>()
                    })
                });

                futures::future::join_all(tasks)
                    .await
                    .into_iter()
                    .map(Result::unwrap)
                    .flatten()
                    .map(|doc| {
                        let header = "{\"index\":{}}".to_owned();
                        let doc = serde_json::to_string(&doc).unwrap();
                        iter::once(header).chain(iter::once(doc))
                    })
                    .flatten()
                    .collect()
            };

            let n_bulk_docs = bulk_body.len() / 2;

            log::debug!("Ingesting {} documents", n_bulk_docs);

            stats.total_indexed += n_bulk_docs as u64; // FIXME: be more pessimistic (check response)

            if !bulk_body.is_empty() {
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

        if let (false, Some(version)) = (leave_old_index, prev_index_version) {
            vna_es::Article::delete_index(elastic, version).await?;
        }

        Ok(stats)
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

fn kaggle_article_to_es_document(article: data_source::kaggle::Article) -> vna_es::Article {
    let sent = sentiment::analyze(article.short_description.clone());

    let (score, polarity) = if sent.negative.score > sent.positive.score {
        (sent.negative.score, vna_es::SentimentPolarity::Negative)
    } else {
        (sent.positive.score, vna_es::SentimentPolarity::Positive)
    };

    vna_es::Article {
        category: article.category,
        headline: article.headline,
        authors: article.authors,
        link: article.link,
        short_description: article.short_description,
        date: article.date,
        sentiment_score: score,
        sentiment_polarity: polarity,
    }
}
