pub mod kaggle {
    use anyhow::Result;
    use serde::Deserialize;
    use std::{
        fs,
        io::{self, BufRead},
        path::Path,
    };

    /// Article from Kaggle news dataset
    #[derive(Deserialize)]
    pub struct Article {
        /// Category article belongs to
        category: String,
        /// Headline of the article
        headline: String,
        /// Person authored the article
        authors: String,
        /// Link to the post
        link: String,
        /// Short description of the article
        short_description: String,
        /// Date the article was published
        date: String,
    }

    impl From<Article> for vna_es::Article {
        fn from(data_source: Article) -> vna_es::Article {
            vna_es::Article {
                category: data_source.category,
                headline: data_source.headline,
                authors: data_source.authors,
                link: data_source.link,
                short_description: data_source.short_description,
                date: data_source.date,
            }
        }
    }

    /// Returns an iterator thru all the articles at the specified `dataset_path`
    /// The file must be in `ndjson` format (i.e. it contains `\n`-delimited json objects).
    /// Each object in the dataset file must conform to the given `Article`
    pub fn read_articles(dataset_path: &Path) -> Result<impl Iterator<Item = Article>> {
        Ok(io::BufReader::new(fs::File::open(dataset_path)?)
            .lines()
            .filter_map(|line| serde_json::from_str(&line.ok()?).ok()?))
    }
}
