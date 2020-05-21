use anyhow::Result;
use elasticsearch::Elasticsearch;
use serde::Serialize;

/// Creates Elasticsearch news documents mapping declration object
fn _create_news_index_mapping() -> impl Serialize {
    todo!()
}

/// Crates news index in elasticsearch.
pub fn create_new_index(_elastic: &mut Elasticsearch, _index_name: &str) -> Result<()> {
    todo!()
}
