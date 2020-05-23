use elasticsearch::cat::CatIndicesParts;
use std::num::NonZeroU32;
use vna_es::{Article, CreateArticlesIndexOpts, IndexVersion};
use vna_es_utils::elasticsearch_client_or_return;

#[tokio::test]
async fn create_index_works() {
    let elastic = elasticsearch_client_or_return!();

    Article::create_index(&CreateArticlesIndexOpts {
        elastic: &elastic,
        number_of_replicas: 0,
        number_of_shards: NonZeroU32::new(1).unwrap(),
        version: IndexVersion::default(),
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

    Article::delete_index(&elastic, IndexVersion::default())
        .await
        .unwrap();
}
