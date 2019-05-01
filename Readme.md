# Kalavara

A distributed persistent key value store which speaks http. Inspired by
[minkeyvalue](https://github.com/geohot/minikeyvalue).


# master server
    Master server stores index (key, url of volume server where the value is
    stored) in rocksdb. Requests are redirected to curresponding volume server
    after metadata is updated.

    to start the server, run

    ```sh
        master -port 6000 -d /tmp/kalavadb -v http://volume1:6001 http://volume2:6002
    ```

## Usage
    1. insert a key-value
       ```sh
       curl -XPUT -L -d value http://localhost:6000/store/key

       ```

    2. retrive value
       ```sh
       curl -XGET -L http://localhost:6000/store/key

       ```

    3. delete a key
       ```sh
       curl -XDELETE -L http://localhost:6000/store/key

       ```
