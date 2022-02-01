import FeatureFormat, {ReadOptions, transformGeometryWithOptions} from "ol/format/Feature";
import {get as getProjection, Projection} from "ol/proj";
import {Feature} from "ol";
import {Geometry, Polygon} from "ol/geom";
import _default from "ol/format/FormatType";
import TEXT = _default.TEXT;
import {H3Index, h3ToGeoBoundary} from 'h3-js';

export default class JsonH3 extends FeatureFormat {
    private readonly h3indexPropertyName: string;

    constructor(h3indexPropertyName: string | undefined) {
        super();
        this.dataProjection = getProjection('EPSG:4326')
        this.h3indexPropertyName = h3indexPropertyName || "h3index";
        this.supportedMediaTypes = [
            "application/json",
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
        JSON.parse(source).forEach((obj: object) => {
            if (obj.hasOwnProperty(this.h3indexPropertyName)) {
                // @ts-ignore
                let h3index = obj[this.h3indexPropertyName].toString() as H3Index;
                // @ts-ignore
                obj['h3index'] = h3index;
                let feature = new Feature<Geometry>()
                feature.setGeometry(this.readGeometry(h3index, opt_options))
                feature.setProperties(obj)
                features.push(feature)
            }
        })
        return features;
    }

    setLayers(layers: any) {}
}
