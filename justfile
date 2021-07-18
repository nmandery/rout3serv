
minio-start:
    mkdir -p ./data/graph ./data/population ./data/outputs
    MINIO_ROOT_USER=admin MINIO_ROOT_PASSWORD=password minio server ./data

fetch-data:
    mkdir -p data
    wget --unlink https://download.geofabrik.de/europe/germany-latest.osm.pbf -O data/germany-latest.osm.pbf

extract-sample-data:
    osmium export --geometry-types 'point' -f geojson -o data/fastfood.geojson -c datasources/osmium.fastfood.json --progress --overwrite data/germany-latest.osm.pbf
    # hospitals
    osmium tags-filter data/germany-latest.osm.pbf wnr/amenity=hospital -o data/hospital.osm.pbf --progress --overwrite
    osmium export --geometry-types 'point' -f geojson -o data/hospital.geojson -c datasources/osmium.hospital.json --progress --overwrite data/hospital.osm.pbf

generate-testdata:
     cargo run --release --bin route3_road -- graph from-osm-pbf -r 7 testdata/graph-germany_r7_f64.bincode data/germany-latest.osm.pbf

build-docker:
    sudo docker build -f route3_road.Dockerfile -t nmandery/route3_road:latest --pull .
