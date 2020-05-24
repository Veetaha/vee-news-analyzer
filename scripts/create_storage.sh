# Precreates the database storage dir with the proper os permissions
# Based on https://www.elastic.co/guide/en/elasticsearch/reference/7.7/docker.html#_configuration_files_must_be_readable_by_the_elasticsearch_user

. .env

ES_NODES="node_1 node_2 node_3 single_node"

for ES_NODE in $ES_NODES; do
    mkdir -p ${VNA_ES_DATA_PATH_ON_HOST}/${ES_NODE}
    chmod g+rwx ${VNA_ES_DATA_PATH_ON_HOST}/${ES_NODE}
    chgrp 0 ${VNA_ES_DATA_PATH_ON_HOST}/${ES_NODE} -f
done
