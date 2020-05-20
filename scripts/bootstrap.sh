# Works only on linux, see other solutions here: https://www.elastic.co/guide/en/elasticsearch/reference/7.7/docker.html#_set_vm_max_map_count_to_at_least_262144
# This workaroudns  the bootstrap error of an elasticsearch node when running docker-compose, which is:
# [1]: max virtual memory areas vm.max_map_count [65530] is too low, increase to at least [262144]
sudo sysctl -w vm.max_map_count=262144
