# Contents

## route3_road

Routing server with GRPC-API and dataframe integration.

Features:

- GRPC API including streamed Arrow IPC recordbatches for tabular data.
- S3 storage backend.
- Builtin extractor to create graphs from OSM-PBF files.
- Python client library.
- Export of graphs to GDAL vector formats.
- LRU-Cache for loaded graphs.
- Dynamic loading of supplementary dataset from S3. 

## til3serv

Experimental tileserver to be used with Web maps in Web-Mercator projection.

**DRAFT**

Features:

- Tiles for Web-Mercator maps in parquet, arrow-ipc, csv and json-lines format.
- S3 storage backend.
- Compression for HTTP responses.

## h3io

Support library shared by the previous applications.

# License

This code is released under a dual Apache 2.0 / MIT free software license.

Data in the `testdata` directory is derived from OpenStreetMap and as such is copyright by the OpenStreetMap contributors. For
the OSM license see [OSMs Copyright and License page](https://www.openstreetmap.org/copyright).
