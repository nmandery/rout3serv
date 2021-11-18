## route3_road

Routing server with GRPC-API and dataframe integration.

Features:

- GRPC API including streamed Arrow IPC recordbatches for tabular data.
- S3 storage backend.
- Builtin extractor to create graphs from OSM-PBF files.
- Python client library.
- Export of graphs to GDAL vector formats.
- In-memory Cache for loaded graphs and datasets.
- Dynamic loading of supplementary dataset from S3.

## til3serv

Tileserver to be used with Web maps in Web-Mercator projection.

Features:

- Tiles for Web-Mercator maps in parquet, arrow-ipc, csv and json-lines format.
- S3 storage backend.
- Compression for HTTP responses.
- In-memory Cache for datasets.
- Bundled web viewer to inspect the tiles.
  - includes simple styling based on a property, value range and color range.

## h3io

Support library shared by the previously listed applications.
