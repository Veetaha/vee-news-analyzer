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
        pub category: String,
        /// Headline of the article
        pub headline: String,
        /// Person authored the article
        pub authors: String,
        /// Link to the post
        pub link: String,
        /// Short description of the article
        pub short_description: String,
        /// Date the article was published
        pub date: String,
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
