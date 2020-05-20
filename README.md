# news-analyzer

![cicd](https://github.com/Veetaha/vee-news-analyzer/workflows/cicd/badge.svg)

[![badge](https://img.shields.io/badge/docs-master-blue.svg)](https://veetaha.github.io/vee-news-analyzer/vna/)


This is my university course work for database classes.

The project uses external APIs to collect current news, stores them
in [`Elasticsearch`](https://github.com/elastic/elasticsearch) cluster and analyzes
the obtained textual information using various `Elasticsearch` analysis APIs
along with other 3-d party services.

# Bootstrap

Deploy local `Elasticsearch` cluster of 3 data nodes on your local machine, also
spin up a `Kibana` instance with auth-less access to that cluster.
```
docker-compose up
```
The configuration tweaks for this is available in `.env` file.
You should create it similar to the provided `EXAMPLE.env` template.



// FIXME: add cli bootstrap guide once it is formed.

# Attribution
<!--
This attribution notice is required by newsapi.org for applications that
use the free plan.
-->
Powered by https://newsapi.org.
