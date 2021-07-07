
minio-start:
    mkdir -p ./data/graph ./data/population ./data/outputs
    MINIO_ROOT_USER=admin MINIO_ROOT_PASSWORD=password minio server ./data

fetch-data:
    mkdir -p data
    wget --unlink https://download.geofabrik.de/europe/germany-latest.osm.pbf -O data/germany-latest.osm.pbf
