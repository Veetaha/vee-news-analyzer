//! vee-news-analyzer cli entrypoint

use anyhow::{anyhow, Result};
use charts::{Chart, ScaleBand, ScaleLinear, VerticalBarView};
use std::{
    num::NonZeroU32,
    ops::Deref,
    path::{Path, PathBuf},
};
use structopt::StructOpt;
use url::Url;
use vna_es_utils::es_types;

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

        /// Maximum amount of news to retain in Elasticsearch database
        #[structopt(long, default_value = "300000")]
        max_news: u64,

        /// Maximum size of the batch to ingest data with into Elasticsearch
        #[structopt(long, default_value = "50000")]
        ingest_batch: NonZeroU32,

        /// Number of shards to use for the Elasticsearch indices (min: 1)
        #[structopt(long, default_value = "1")]
        n_shards: NonZeroU32,

        /// Number of relicas to create for the Elasticsearch indices
        #[structopt(long, default_value = "0")]
        n_replicas: u32,

        /// Whether to delete old index once the new one is created,
        #[structopt(long)]
        leave_old_index: bool,

        #[structopt(flatten)]
        elasticsearch: ElasticsearchArgs,

        #[structopt(flatten)]
        data_source: DataSourceArgs,
    },

    /// View varios statistics about the news via SVG charts
    Stats {
        #[structopt(flatten)]
        kind: StatsKind,

        #[structopt(flatten)]
        elasticsearch: ElasticsearchArgs,
    },

    /// Issue a fulltext search thru all the news
    Search {
        /// String of text to search for in Elasticsearch
        query: stdx::NonHollowString,

        /// Particular name of the field to search by in elasticsearch.
        /// If none is specified (which is the default) searches by all
        /// text fields
        #[structopt(long)]
        field_name: Option<String>,

        #[structopt(flatten)]
        elasticsearch: ElasticsearchArgs,
    },
}

#[derive(Debug, StructOpt)]
enum StatsKind {
    /// Display significant words statistics for a particular query
    SignificantWords {
        /// Query that will be used as a root to find related significant words for
        query: stdx::NonHollowString,

        /// Particular name of the field to search by in elasticsearch.
        #[structopt(long, default_value = "short_description")]
        field_name: String,

        /// Maximym number of significant words to return
        #[structopt(long, default_value = "15")]
        max_words: u32,
    }, // /// Display news trends graphs
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
struct DataSourceArgs {
    /// Path to kaggle news dataset
    #[structopt(long, env = "VNA_KAGGLE_PATH")]
    kaggle_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(err) = dotenv::dotenv() {
        log::debug!("Dotenv could not be loaded: {:?}", err);
    }

    env_logger::init();

    let cli_args = CliArgs::from_args();

    log::debug!("Using cli args: {:?}", cli_args);

    match cli_args {
        CliArgs::DataSync {
            max_news,
            scrape_interval,
            elasticsearch,
            data_source,
            n_replicas,
            n_shards,
            ingest_batch,
            leave_old_index,
        } => {
            eprintln!("Running data sync task...");
            let time = std::time::Instant::now();
            let stats = vna_data_sync::run(vna_data_sync::RunOpts {
                es_url: elasticsearch.es_url,
                kaggle_dataset_path: &data_source.kaggle_path,
                scrape_interval,
                max_news,
                n_replicas,
                n_shards,
                ingest_batch,
                leave_old_index,
            })
            .await?;
            eprintln!(
                "Data sync task has finished\n\
                took: {:?},\n\
                new_index_name: {},\n\
                total_indexed: {}\n",
                time.elapsed(),
                stats.new_index_name,
                stats.total_indexed,
            );
        }
        CliArgs::Search {
            field_name,
            query,
            elasticsearch,
        } => {
            eprintln!("Searching for articles...");

            let elastic = &vna_es_utils::create_elasticsearch_client(elasticsearch.es_url)?;

            let articles = vna_es::Article::fulltext_search(vna_es::FulltextSearchOpts {
                elastic,
                field_name: field_name.as_deref(),
                query: &query,
            })
            .await?;

            eprintln!("Found articles {}", articles.len());
            for article in articles {
                eprintln!("{:#?}", article);
            }
        }
        CliArgs::Stats {
            kind,
            elasticsearch,
        } => match kind {
            StatsKind::SignificantWords {
                query,
                max_words,
                field_name,
            } => {
                let elastic = &vna_es_utils::create_elasticsearch_client(elasticsearch.es_url)?;
                let time = std::time::Instant::now();
                let result: es_types::SignificantTextAggr =
                    vna_es::Article::significant_words(vna_es::SignificantWordsOpts {
                        elastic,
                        field_name: &field_name,
                        query: &query,
                        max_words,
                    })
                    .await?;

                let chart_path = Path::new("./significant_words.svg");

                eprintln!(
                    "Total docs: {} in {:?}, words: {}",
                    result.doc_count,
                    time.elapsed(),
                    result.buckets.len()
                );

                if result.buckets.len() > 0 {
                    create_significant_words_chart(&result.buckets, &query, chart_path)?;

                    std::process::Command::new("google-chrome")
                        .arg(chart_path)
                        .spawn()?
                        .wait()?;
                }
            }
        },
    }

    Ok(())
}

fn create_significant_words_chart(
    words: &[es_types::SignificantTextAggrBucket],
    query: &stdx::NonHollowString,
    file_path: &Path,
) -> Result<()> {
    let width = 1500;
    let height = 900;
    let (top, right, bottom, left) = (90, 40, 200, 60);

    let x = ScaleBand::new()
        .set_domain(words.iter().map(|it| it.key.clone()).collect())
        .set_range(vec![0, width - left - right])
        .set_inner_padding(0.1)
        .set_outer_padding(0.1);

    let max = words.iter().map(|it| it.doc_count).max().unwrap();

    let y = ScaleLinear::new()
        .set_domain(vec![0.0, max as f32])
        .set_range(vec![height - top - bottom, 0]);

    // You can use your own iterable as data as long as its items implement the `BarDatum` trait.
    let data = words
        .iter()
        .map(|it| (it.key.as_str(), it.doc_count as f32))
        .collect();

    // Create VerticalBar view that is going to represent the data as vertical bars.
    let view = VerticalBarView::new()
        .set_x_scale(&x)
        .set_y_scale(&y)
        .set_colors(charts::Color::color_scheme_dark())
        .load_data(&data)
        .map_err(|err| anyhow!("{}", err))?;

    // Generate and save the chart.
    Chart::new()
        .set_width(width)
        .set_height(height)
        .set_margins(top, right, bottom, left)
        .add_title(format!("Significant words ({})", query.deref()))
        .add_view(&view)
        .add_axis_bottom(&x)
        .add_axis_left(&y)
        .add_left_axis_label("Total news with the word")
        .add_bottom_axis_label("Significant words")
        .save(file_path)
        .map_err(|err| anyhow!("{}", err))?;

    Ok(())
}
