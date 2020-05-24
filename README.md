[kaggle-dataset]: https://www.kaggle.com/rmisra/news-category-dataset

# vee-news-analyzer

![cicd](https://github.com/Veetaha/vee-news-analyzer/workflows/cicd/badge.svg)
[![badge](https://img.shields.io/badge/docs-master-blue.svg)](https://veetaha.github.io/vee-news-analyzer/vna/)


This is my university course work for database classes.

The project uses [Kaggle news dataset][kaggle-dataset].

The core of the project is `vna` cli.
It reads the provided **`~200K`** articles from the data source and puts them into [`Elasticsearch`](https://github.com/elastic/elasticsearch) cluster while analyzing
the obtained textual information sentiment.
The ingest process is implemented by `vna_data_sync` component.
After this process is done you can use `vna` cli to do fulltext search,
query significant words statistics and news sentiments using various `Elasticsearch`
analysis and query APIs.

# Example analytics

## Significant words

```bash
vna stats significant-words study
```

![significant_words](https://user-images.githubusercontent.com/36276403/82747264-e4128000-9d9f-11ea-8a96-4b167013dc13.png)

```bash
vna stats sentiment study
```

![sentiment](https://user-images.githubusercontent.com/36276403/82749758-1c23be00-9db4-11ea-9302-631ab37f70dc.png)

```bash
vna stats category study
```

![category](https://user-images.githubusercontent.com/36276403/82750418-aff78900-9db8-11ea-82c6-22044ad292c9.png)


# Bootstrap

## Elasticsearch and Kibana

Deploy local `Elasticsearch` cluster on your local machine, also
spin up a `Kibana` instance with auth-less access to that cluster.
```bash
# You should run these two workaround-commands only once and never more (at least I hope so)
./scripts/create_storage.sh
./scripts/bootstrap.sh
# FIXME: add kaggle dataset download and unzip command to scripts...

# Deploy 3-data-nodes cluster
docker-compose -f docker-compose-multi-node.yml up

# Deploy single-node cluster (best for development, otherwise Java will eat your RAM)
docker-compose up
```
The configuration tweaks for the setup are available in `.env` file.
You should create it similar to the provided `EXAMPLE.env` template.

## Cli

Build the cli. The self-contained executable will be available at `./target/(debug|release)/vna`

```bash
# debug build
cargo build

# release build
cargo build --release
```

Build and run the cli ingest process. Be sure to put the [kaggle dataset][kaggle-dataset]
at `./datasets/kaggle/news_v2.json` or anywhere else (just make sure that `--kaggle-path`
points to it).

```bash
cargo run [--release] -p vna -- data-sync
```

When run in release mode and with the single-node Elasticsearch cluster `data-sync` process
takes approx. `50 seconds` for `~200K` documents on my laptop
- `Lenovo 520`
- `Intel Core i7-8550U CPU @ 1.80GHz × 8`
- `8GB RAM`

View the help info via
```
vna [[--]help] [subcommand]
```

At the time of writing this it looks like this:

```
vee-news-analyzer 0.1.0

USAGE:
    vna --es-url <es-url> <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --es-url <es-url>    Elasticsearch endpoint url to use [env: VNA_ES_URL=http://127.0.0.1:9200]

SUBCOMMANDS:
    data-sync    Run data synchronization job that will use the external data source
    help         Prints this message or the help of the given subcommand(s)
    search       Issue a fulltext search thru all the news
    snapshots    Elasticsearch snapshots management commands
    stats        View varios statistics about the news via SVG charts
```
