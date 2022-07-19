/*
requires "@apache-arrow/es5-cjs": "^7.0.0"
*/

import {ReadOptions} from "ol/format/Feature";
import {Feature} from "ol";
import {Geometry} from "ol/geom";
import {Table, tableFromIPC} from "@apache-arrow/es5-cjs";
import H3FeatureFormat from "./base";

function h3indexToString(h3index: number) : string {
    return h3index.toString(16).padEnd(15, "f");
}

/**
 * Notes:
 * * js arrow 7 does not yet support LargeUTF8 (used by polars for strings
 */
export default class ArrowH3 extends H3FeatureFormat {

    constructor(h3indexPropertyName: string | undefined) {
        super(h3indexPropertyName);
        this.supportedMediaTypes = [
            "application/vnd.apache.arrow.file",
        ]
    }

    getType(): any {
        return 'arraybuffer';
    }

    readFeatures(source: ArrayBuffer, opt_options?: ReadOptions | undefined): Feature<Geometry>[] {
        if (source.byteLength == 0) {
            return [];
        }
        let table: Table = tableFromIPC(source);
        //console.log(table);
        //console.log(table.schema.names);

        opt_options = this.getReadOptions(source, opt_options)
        let features: Feature<Geometry>[] = [];
        let properties: Object[] = [];
        const h3indexColumn = table.getChild(this.h3indexPropertyName);
        for (let i = -1, n = h3indexColumn.length; ++i < n;) {
            let h3indexString = h3indexToString(h3indexColumn.get(i));
            let feature = new Feature<Geometry>({
                geometry: this.readGeometry(h3indexString, opt_options)
            });
            features.push(feature);
            let props = {};
            // @ts-ignore
            props[this.h3indexPropertyName] = h3indexString;
            properties.push(props);
        }

        if (features.length > 0) {
            for (let columnI = 0; columnI < table.schema.names.length; columnI++) {
                const columnName = table.schema.names[columnI];
                const column = table.getChild(columnName);
                if (columnName != this.h3indexPropertyName) {
                    for (let i = -1, n = column.length; ++i < n;) {
                        // @ts-ignore
                        properties[i][columnName] = column.get(i);
                    }
                }
            }
            for (let i = -1, n = features.length; ++i < n;) {
                features[i].setProperties(properties[i]);
            }
        }
        return features;
    }
}
