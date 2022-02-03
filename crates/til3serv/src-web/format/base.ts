import FeatureFormat, {ReadOptions, transformGeometryWithOptions} from "ol/format/Feature";
import {get as getProjection, Projection} from "ol/proj";
import {H3Index, h3ToGeoBoundary} from "h3-js";
import {Geometry, Polygon} from "ol/geom";

export default class H3FeatureFormat extends FeatureFormat {

    protected readonly h3indexPropertyName: string;

    constructor(h3indexPropertyName: string | undefined) {
        super();
        this.dataProjection = getProjection('EPSG:4326')
        this.h3indexPropertyName = h3indexPropertyName || "h3index";
    }

    readProjection(source: any): Projection {
        return getProjection('EPSG:4326');
    }

    readGeometry(h3index: H3Index, opt_options?: ReadOptions | undefined): Geometry {
        let extRing = h3ToGeoBoundary(h3index, true)
        let geom = new Polygon([extRing],)
        return transformGeometryWithOptions(geom, false, opt_options)
    }

    setLayers(layers: any) {
    }
}
