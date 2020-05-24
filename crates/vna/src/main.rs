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
struct CliArgs {
    /// Elasticsearch endpoint url to use
    #[structopt(long, env = "VNA_ES_URL")]
    es_url: Url,

    #[structopt(flatten)]
    subcommand: CliSubcommand,
}

#[derive(Debug, StructOpt)]
enum CliSubcommand {
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
        data_source: DataSourceArgs,
    },

    /// View varios statistics about the news via SVG charts
    Stats(Stats),

    /// Issue a fulltext search thru all the news
    Search {
        /// String of text to search for in Elasticsearch
        query: stdx::NonHollowString,

        /// Particular name of the field to search by in elasticsearch.
        /// If none is specified (which is the default) searches by all
        /// text fields
        #[structopt(long)]
        field_name: Option<String>,
    },
}

#[derive(Debug, StructOpt)]
enum Stats {
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

        /// Path where to put the rendered SVG chart
        #[structopt(long, default_value = "./significant_words.svg")]
        chart_path: PathBuf,
    },
    /// Display the sentiment analysis statistics for the given
    /// subset of documents filtered by the query string or for all news
    /// altogether
    Sentiment {
        /// Query that will be used to filter the documents to aggregate sentiment
        /// info. If not specified returns the sentiment for all the news in Elasticsearch
        query: Option<stdx::NonHollowString>,

        /// Particular name of the field to search by in elasticsearch.
        #[structopt(long, default_value = "short_description")]
        field_name: String,

        /// Path where to put the rendered SVG chart
        #[structopt(long, default_value = "./sentiment.svg")]
        chart_path: PathBuf,
    },
    /// Display the category statistics for the given
    /// subset of documents filtered by the query string or for all news
    /// altogether
    Category {
        /// Query that will be used to filter the documents to aggregate category
        /// info. If not specified returns the category info for all the news in Elasticsearch
        query: Option<stdx::NonHollowString>,

        /// Particular name of the field to search by in elasticsearch.
        #[structopt(long, default_value = "short_description")]
        field_name: String,

        /// Path where to put the rendered SVG chart
        #[structopt(long, default_value = "./category.svg")]
        chart_path: PathBuf,
    },
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

    let elastic = &vna_es_utils::create_elasticsearch_client(cli_args.es_url)?;

    match cli_args.subcommand {
        CliSubcommand::DataSync {
            max_news,
            scrape_interval,
            data_source,
            n_replicas,
            n_shards,
            ingest_batch,
            leave_old_index,
        } => {
            eprintln!("Running data sync task...");
            let time = std::time::Instant::now();
            let stats = vna_data_sync::run(vna_data_sync::RunOpts {
                elastic,
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
        CliSubcommand::Search { field_name, query } => {
            eprintln!("Searching for articles...");

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
        CliSubcommand::Stats(stats) => match stats {
            Stats::SignificantWords {
                query,
                max_words,
                field_name,
                chart_path,
            } => {
                let time = std::time::Instant::now();
                let result: es_types::SignificantTextAggr =
                    vna_es::Article::significant_words(vna_es::SignificantWordsOpts {
                        elastic,
                        field_name: &field_name,
                        query: &query,
                        max_words,
                    })
                    .await?;

                eprintln!(
                    "Total docs: {} in {:?}, words: {}",
                    result.doc_count,
                    time.elapsed(),
                    result.buckets.len()
                );

                if result.buckets.len() > 0 {
                    create_significant_words_chart(&result.buckets, &query, &chart_path)?;
                    open_svg_in_google_chrome(&chart_path)?;
                }
            }
            Stats::Sentiment {
                field_name,
                query,
                chart_path,
            } => {
                let stats = vna_es::Article::sentiment_stats(vna_es::StatsOpts {
                    elastic,
                    field_name: &field_name,
                    query: &query,
                })
                .await?;

                create_sentiment_analysis_chart(&query, &chart_path, stats)?;
                open_svg_in_google_chrome(&chart_path)?;
            }
            Stats::Category {
                field_name,
                query,
                chart_path,
            } => {
                let stats = vna_es::Article::category_stats(vna_es::StatsOpts {
                    elastic,
                    field_name: &field_name,
                    query: &query,
                })
                .await?;

                dbg!(&stats.0);

                create_category_analysis_chart(&query, &chart_path, stats)?;
                open_svg_in_google_chrome(&chart_path)?;
            }
        },
    }

    Ok(())
}

fn create_category_analysis_chart(
    query: &Option<stdx::NonHollowString>,
    file_path: &Path,
    mut stats: vna_es::Stats,
) -> Result<()> {
    create_chart(ChartOpts {
        title: match query {
            Some(it) => format!("Categories stats ({})", it.deref()),
            None => format!("Categories stats"),
        },
        left_axis_label: "Total news with the category",
        bottom_axis_label: "Categories",
        color: charts::Color::from_vec_of_hex_strings(vec!["#c56969"]),
        path: file_path,
        data: stats_to_chart_data(&mut stats),
    })
}

fn create_sentiment_analysis_chart(
    query: &Option<stdx::NonHollowString>,
    file_path: &Path,
    mut stats: vna_es::Stats,
) -> Result<()> {
    create_chart(ChartOpts {
        title: match query {
            Some(it) => format!("Sentiments stats ({})", it.deref()),
            None => format!("Sentiments stats"),
        },
        left_axis_label: "Total news with the sentiment",
        bottom_axis_label: "Sentiments",
        color: charts::Color::from_vec_of_hex_strings(vec!["#e81e31"]),
        path: file_path,
        data: stats_to_chart_data(&mut stats),
    })
}

fn stats_to_chart_data(stats: &mut vna_es::Stats) -> Vec<(&str, f32)> {
    for (name, _) in stats.0.iter_mut() {
        *name = name.replace("&", "and"); // FIXME: do real XML escaping here
    }

    stats
        .0
        .iter()
        .map(|(name, val)| (name.as_str(), *val as f32))
        .collect()
}

fn create_significant_words_chart(
    words: &[es_types::SignificantTextAggrBucket],
    query: &stdx::NonHollowString,
    file_path: &Path,
) -> Result<()> {
    create_chart(ChartOpts {
        title: format!("Significant words ({})", query.deref()),
        left_axis_label: "Total news with the word",
        bottom_axis_label: "Significant words",
        color: charts::Color::color_scheme_dark(),
        path: file_path,
        data: words
            .iter()
            .map(|it| (it.key.as_str(), it.doc_count as f32))
            .collect(),
    })
}

fn open_svg_in_google_chrome(path: &Path) -> Result<()> {
    std::process::Command::new("google-chrome")
        .arg(path)
        .spawn()?
        .wait()?;
    Ok(())
}

struct ChartOpts<'a> {
    title: String,
    left_axis_label: &'a str,
    bottom_axis_label: &'a str,
    path: &'a Path,
    data: Vec<(&'a str, f32)>,
    color: Vec<charts::Color>,
}

fn create_chart(opts: ChartOpts<'_>) -> Result<()> {
    let width = 1500;
    let height = 900;
    let (top, right, bottom, left) = (90, 40, 200, 60);

    let x = ScaleBand::new()
        .set_domain(opts.data.iter().map(|(cat, _)| cat.to_string()).collect())
        .set_range(vec![0, width - left - right])
        .set_inner_padding(0.1)
        .set_outer_padding(0.1);

    let max = opts
        .data
        .iter()
        .map(|(_, it)| *it)
        .max_by(|a, b| a.partial_cmp(b).expect("Tried to compare a NaN"))
        .unwrap();

    let y = ScaleLinear::new()
        .set_domain(vec![0.0, max])
        .set_range(vec![height - top - bottom, 0]);

    // Create VerticalBar view that is going to represent the data as vertical bars.
    let view = VerticalBarView::new()
        .set_x_scale(&x)
        .set_y_scale(&y)
        .set_colors(opts.color)
        .load_data(&opts.data)
        .map_err(|err| anyhow!("{}", err))?;

    // Generate and save the chart.
    Chart::new()
        .set_width(width)
        .set_height(height)
        .set_margins(top, right, bottom, left)
        .add_title(opts.title)
        .add_view(&view)
        .add_axis_bottom(&x)
        .add_axis_left(&y)
        .add_left_axis_label(opts.left_axis_label)
        .add_bottom_axis_label(opts.bottom_axis_label)
        .save(opts.path)
        .map_err(|err| anyhow!("{}", err))?;

    Ok(())
}
