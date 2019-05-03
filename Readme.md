# Kalavara

A distributed persistent key value store that speaks http. Inspired by
[minkeyvalue](https://github.com/geohot/minikeyvalue).


# master server

Master server stores index (key, url of volume server where the value is
stored) in rocksdb. Requests are redirected to curresponding volume server
after metadata is updated.

to start the server, run

```sh
master -p 6000 -d /tmp/kalavadb -v http://volume1:6001 http://volume2:6002
```

# volume server

Volume server stores values in file system. For atomicity temporary files are
first created in `destdir/tmp` directory and then moved to destination path.
For this approach to work, `destdir/tmp` and destination path should be in same
file system

to start the volume server, run

```sh
master -p 7000 -d /tmp/kalavarastore
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

# License

<p xmlns:dct="http://purl.org/dc/terms/"
  xmlns:vcard="http://www.w3.org/2001/vcard-rdf/3.0#"> <a rel="license"
  href="http://creativecommons.org/publicdomain/zero/1.0/"> <img
  src="http://i.creativecommons.org/p/zero/1.0/88x31.png" style="border-style:
  none;" alt="CC0" /> </a> <br />

To the extent possible under law, the author(s) have dedicated all copyright
related and neighboring rights to this software to the public domain worldwide.
This software is distributed without any warranty.

</p>
