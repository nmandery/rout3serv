# til3serv

Tileserver to be used with Web maps in Web-Mercator projection.

Features:

- Tiles for Web-Mercator maps in parquet, arrow-ipc, csv and json-lines format.
- S3 storage backend.
- Compression for HTTP responses.
- In-memory Cache for datasets.
- Bundled web viewer to inspect the tiles.
  - includes simple styling based on a property, value range and color range.

Configuration: [config.example.yaml](config.example.yaml)
