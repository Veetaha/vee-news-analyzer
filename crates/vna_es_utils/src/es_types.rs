use serde::Deserialize;
use std::collections::HashMap;

// FIXME: there are definitely more fields in response objects
// (deserialization might fail when other fields are present too?)

#[derive(Deserialize)]
pub struct GetAliasesResponse(pub HashMap<String, IndexAliases>);

#[derive(Deserialize)]
pub struct IndexAliases {
    pub aliases: HashMap<String, IndexAlias>,
}

#[derive(Deserialize)]
pub struct IndexAlias {
    // TODO: fill in
}

#[test]
fn get_aliases_response_works() {
    serde_json::from_value::<GetAliasesResponse>(serde_json::json!({
        ".kibana_1": { "aliases": { ".kibana": {} } },
        ".kibana_task_manager_1": { "aliases": { ".kibana_task_manager": {} } },
        ".apm-agent-configuration": { "aliases": {} },
        ".apm-custom-link": { "aliases": {} }
    }))
    .unwrap();
}

#[derive(Deserialize)]
pub struct Doc<Entity> {
    pub _id: String,
    pub _source: Entity,
}

#[derive(Deserialize)]
pub struct SearchHits<Entity> {
    pub hits: Vec<Doc<Entity>>,
}

#[derive(Deserialize)]
pub struct SearchResponse<Entity> {
    // FIXME: fully fill in
    pub hits: SearchHits<Entity>,
}

#[derive(Deserialize)]
pub struct SignificantTextSearch {
    pub aggregations: SignificantTextAggrs,
}
#[derive(Deserialize)]
pub struct SignificantTextAggrs {
    pub keywords: SignificantTextAggr,
}

#[derive(Deserialize)]
pub struct SignificantTextAggr {
    pub doc_count: u64,
    pub bg_count: u64,
    pub buckets: Vec<SignificantTextAggrBucket>,
}

#[derive(Deserialize)]
pub struct SignificantTextAggrBucket {
    pub key: String,
    pub doc_count: u64,
    pub score: f64,
    pub bg_count: u64,
}
