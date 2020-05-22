use newsapi::{api::Client as NewsApiClient, payload::article::Articles};

pub(crate) struct ArticlesPagination<'a> {
    api_key: &'a str,
    next_page_index: u32,
    total: Option<usize>,
}

impl ArticlesPagination<'_> {
    const MAX_PAGE_SIZE: u32 = 100;

    pub(crate) fn new(api_key: &str) -> ArticlesPagination<'_> {
        ArticlesPagination {
            api_key,
            next_page_index: 1,
            total: None,
        }
    }
}

impl Iterator for ArticlesPagination<'_> {
    type Item = Articles;
    fn next(&mut self) -> Option<Articles> {
        if matches!(self.total, Some(total) if Self::MAX_PAGE_SIZE * (self.next_page_index - 1) >= total as u32)
        {
            return None;
        }

        let mut client = NewsApiClient::new(self.api_key.to_owned());
        client
            .page(self.next_page_index)
            .page_size(Self::MAX_PAGE_SIZE)
            .query("*")
            .everything();

        let articles: Articles = client.send().unwrap_or_else(|err| {
            panic!("Error while paginating thru the news api: {:?}", err);
        });

        log::debug!("{:?}", articles);

        self.total = Some(articles.total_results);
        self.next_page_index += 1;

        Some(articles)
    }
}
