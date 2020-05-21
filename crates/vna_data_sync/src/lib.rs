use anyhow::Result;
use elasticsearch::{
    http::transport::{SingleNodeConnectionPool, TransportBuilder},
    Elasticsearch,
};
use newsapi::payload::article::Articles;
use url::Url;

// const:

pub struct Opts {
    pub es_url: Url,
    pub news_api_key: String,
    pub scrape_interval: Option<u32>,
    pub max_news: u64,
}

pub fn run(opts: Opts) -> Result<()> {
    let mut elastic = {
        let conn_pool = SingleNodeConnectionPool::new(opts.es_url);
        let transport = TransportBuilder::new(conn_pool).disable_proxy().build()?;
        Elasticsearch::new(transport)
    };

    let scrape_interval = match opts.scrape_interval {
        Some(it) => it,
        None => {
            scrape(&mut elastic, opts.news_api_key, opts.max_news)?;
            return Ok(());
        }
    };
    let scrape_interval = std::time::Duration::from_millis(scrape_interval as u64);

    loop {
        scrape(&mut elastic, opts.news_api_key.clone(), opts.max_news)?;
        std::thread::sleep(scrape_interval);
    }
}

fn scrape(_elastic: &mut Elasticsearch, news_api_key: String, max: u64) -> Result<()> {
    let _t = stdx::debug_time_it("Scraping news api");

    let mut news_client = newsapi::api::Client::new(news_api_key);

    // TODO: Create new documents index and set the alias to it, afterwards delete the old index
    //

    let mut _total_saved = 0_u64;
    while _total_saved < max {
        let _articles: Articles = news_client.everything().send()?;

        // total_saved += todo!();
    }

    Ok(())
}
