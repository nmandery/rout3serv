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

## Design considerations

### Load reduction through optimized storage

Currently, the data is stored in the backend grouped by h3 cell. For serving rectangular tiles the required cells have to
be filtered out from this, as this depends on running `polyfill` it is a somewhat expensive operation (up to 70ms/tile). Having
the backend store in pre-filtered tiles would reduce load. The tileserver would then only apply attribute filters.
