use elasticsearch::{cat::CatIndicesParts, indices::IndicesDeleteParts, Elasticsearch};
use vna_elasticsearch::{create_articles_index, CreateArticlesIndexOpts};

fn try_create_elasticsearch_client() -> Option<Elasticsearch> {
    let url: url::Url = std::env::var("VNA_TEST_ES_URL").ok()?.parse().unwrap();
    Some(vna_elasticsearch::create_elasticsearch_client(url).unwrap())
}

macro_rules! elasticsearch_client_or_return {
    () => {
        match try_create_elasticsearch_client() {
            Some(it) => it,
            None => return,
        }
    };
}

#[tokio::test]
async fn create_index_works() {
    let elastic = elasticsearch_client_or_return!();

    create_articles_index(&CreateArticlesIndexOpts {
        elastic: &elastic,
        number_of_replicas: 0,
        number_of_shards: 1,
        version: 1,
    })
    .await
    .unwrap();

    let res = elastic
        .cat()
        .indices(CatIndicesParts::Index(&["articles_v1"]))
        .format("json")
        .send()
        .await
        .unwrap();

    let res: serde_json::Value = res.json().await.unwrap();

    assert_eq!(res[0]["index"], serde_json::json! {"articles_v1"});
    assert_eq!(res[0]["health"], serde_json::json! {"green"});

    elastic
        .indices()
        .delete(IndicesDeleteParts::Index(&["articles_v1"]))
        .send()
        .await
        .unwrap();
}
