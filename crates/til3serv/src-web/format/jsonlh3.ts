import {ReadOptions} from "ol/format/Feature";
import {Feature} from "ol";
import {Geometry} from "ol/geom";
import JSONL from "jsonl-parse-stringify";
import {H3Index} from 'h3-js';
import H3FeatureFormat from "./base";

export default class JsonLH3 extends H3FeatureFormat {

    constructor(h3indexPropertyName: string | undefined) {
        super(h3indexPropertyName);
        this.supportedMediaTypes = [
            "application/jsonlines+json",
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
        JSONL.parse<object>(source).forEach((obj) => {
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
