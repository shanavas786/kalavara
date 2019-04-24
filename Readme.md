# Kalavara

A distributed file based key value store inspired by [minkeyvalue]( https://github.com/geohot/minikeyvalue )
It uses rocksdb unlike minkeyvalue which uses leveldb as rust binding for
leveldb currently supports only integer keys


# master server
    master --port 3000 --dbdir /tmp/kalavadb
