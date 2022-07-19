import {ReadOptions} from "ol/format/Feature";
import {Feature} from "ol";
import {Geometry} from "ol/geom";
import {H3Index} from 'h3-js';
import H3FeatureFormat from "./base";

export default class JsonH3 extends H3FeatureFormat {
    constructor(h3indexPropertyName: string | undefined) {
        super(h3indexPropertyName);
        this.supportedMediaTypes = [
            "application/json",
        ]
    }
    getType(): any {
        return "text";
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
}
