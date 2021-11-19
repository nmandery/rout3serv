 /*
initial draft. blocked by uint64 support.
 requires "@apache-arrow/es5-cjs": "^6.0.0",

import FeatureFormat, {ReadOptions} from "ol/format/Feature";
import {get as getProjection, Projection} from "ol/proj";
import {Feature} from "ol";
import {Polygon} from "ol/geom";
import {Table} from "@apache-arrow/es5-cjs";
import _default from "ol/format/FormatType";
import ARRAY_BUFFER = _default.ARRAY_BUFFER;

export default class H3Arrow extends FeatureFormat {
    getType(): any {
        return ARRAY_BUFFER;
    }

    readProjection(source: any): Projection {
        return getProjection('epsg:4326');
    }

    readFeatures(source: ArrayBuffer, opt_options?: ReadOptions | undefined): Feature<Polygon>[] {
        if (source.byteLength == 0) {
            return [];
        }
        console.log(source);
        let table = Table.from(source);
        let features = [];
        console.log(table.getColumn("h3index").toArray())
        console.log(table.getColumn("h3index").type)
        console.log(table.getColumn("h3index").type.ArrayType)
        console.log(table.toString())
        return [];
    }

    setLayers(layers: any) {}
}
*/
