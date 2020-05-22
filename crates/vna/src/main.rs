//! vee-news-analyzer cli entrypoint

use anyhow::Result;
use std::{num::NonZeroU32, str::FromStr};
use structopt::StructOpt;
use url::Url;

#[structopt(name = "vee-news-analyzer")]
#[derive(Debug, StructOpt)]
enum CliArgs {
    /// Run data synchronization job that will call out to external APIs that
    /// provide news data (currently this is only https://newsapi.org/)
    DataSync {
        /// Number of milliseconds to wait between successive scrapes of the
        /// external APIs. If none is specified the job is run only one time
        /// and shutdown right away, otherwise it will sleep for the specified
        /// amount of time and do the scraping inifinitely.
        #[structopt(long)]
        scrape_interval: Option<u32>,

        /// Maximum amount of news to retain in Elasticsearch database.
        #[structopt(long, default_value = "100000")]
        max_news: u64,

        /// Number of shards to use for the Elasticsearch indices (min: 1)
        #[strctopt(long, default_value = "1")]
        n_shards: NonZeroU32,

        /// Number of relicas to create for the Elasticsearch indices
        #[structopt(long, default_value = "0")]
        n_replicas: u32,

        #[structopt(flatten)]
        elasticsearch: ElasticsearchArgs,

        #[structopt(flatten)]
        news_api: NewsApiArgs,
    },

    Stats {
        #[structopt(flatten)]
        kind: StatsKind,

        #[structopt(flatten)]
        elasticsearch: ElasticsearchArgs,
    },
}
#[derive(Debug, StructOpt)]
enum StatsKind {
    // /// Display news trends graphs
// Trends,

// /// Display news sentiment analyzis statistics
// Sentiment {
//     news_id: String,
// }
}

#[derive(Debug, StructOpt)]
struct ElasticsearchArgs {
    /// Elasticsearch endpoint url to use
    #[structopt(long, env = "VNA_ES_URL")]
    es_url: Url,
}

#[derive(Debug, StructOpt)]
struct NewsApiArgs {
    /// Api key obtained from https://newsapi.org
    #[structopt(long, env = "VNA_NEWS_API_KEY")]
    news_api_key: String,
}

fn main() -> Result<()> {
    if let Err(err) = dotenv::dotenv() {
        log::debug!("Dotenv could not be loaded: {:?}", err);
    }

    env_logger::init();

    std::path::PathBuf::from_str("").unwrap();

    let cli_args = CliArgs::from_args();

    log::debug!("Using cli args: {:?}", cli_args);

    match cli_args {
        CliArgs::DataSync {
            max_news,
            scrape_interval,
            elasticsearch,
            news_api,
            n_replicas,
            n_shards,
        } => {
            eprintln!("Running data sync task...");
            vna_data_sync::run(vna_data_sync::RunOpts {
                es_url: elasticsearch.es_url,
                news_api_key: news_api.news_api_key,
                scrape_interval,
                max_news,
            })?;
            eprintln!("Data sync task has finished")
        }
        CliArgs::Stats { .. } => {
            todo!();
        }
    }

    Ok(())
}
