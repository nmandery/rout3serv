import FeatureFormat, {ReadOptions, transformGeometryWithOptions} from "ol/format/Feature";
import {get as getProjection, Projection} from "ol/proj";
import {Feature} from "ol";
import {Geometry, Polygon} from "ol/geom";
import _default from "ol/format/FormatType";
import TEXT = _default.TEXT;
import JSONL from "jsonl-parse-stringify";
import {H3Index, h3ToGeoBoundary} from 'h3-js';

interface H3IndexedObject {
    h3index: H3Index
}

export default class JsonLH3 extends FeatureFormat {
    constructor() {
        super();
        this.dataProjection = getProjection('EPSG:4326')
        this.supportedMediaTypes = [
            "application/jsonlines+json",
        ]
    }
    getType(): any {
        return TEXT;
    }

    readProjection(source: any): Projection {
        return getProjection('EPSG:4326')
    }

    readGeometry(h3index: H3Index, opt_options?: ReadOptions | undefined): Geometry {
        let extRing = h3ToGeoBoundary(h3index, true)
        let geom = new Polygon([extRing], )
        return transformGeometryWithOptions(geom, false, opt_options)
    }

    readFeatures(source: string, opt_options?: ReadOptions | undefined): Feature<Geometry>[] {
        if (source.length == 0) {
            return [];
        }
        opt_options = this.getReadOptions(source, opt_options)
        let features: Feature<Geometry>[] = []
        JSONL.parse<H3IndexedObject>(source).forEach((h3indexObj) => {
            let feature = new Feature<Geometry>()
            feature.setGeometry(this.readGeometry(h3indexObj.h3index, opt_options))
            feature.setProperties(h3indexObj)
            features.push(feature)
        })
        return features;
    }

    setLayers(layers: any) {}
}
