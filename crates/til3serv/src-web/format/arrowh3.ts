/*
initial draft. blocked by uint64 support.
requires "@apache-arrow/es5-cjs": "^6.0.0",
*/

import FeatureFormat, {ReadOptions, transformGeometryWithOptions} from "ol/format/Feature";
import {get as getProjection, Projection} from "ol/proj";
import {Feature} from "ol";
import {Geometry, Polygon} from "ol/geom";
import {Table, tableFromIPC} from "@apache-arrow/es5-cjs";
import _default from "ol/format/FormatType";
import {H3Index, h3ToGeoBoundary} from "h3-js";
import ARRAY_BUFFER = _default.ARRAY_BUFFER;

function h3indexToString(h3index: number) : string {
    return h3index.toString(16).padEnd(15, "f");
}

/**
 * Notes:
 * * js arrow 7 does not yet support LargeUTF8 (used by polars for strings
 */
export default class ArrowH3 extends FeatureFormat {

    private readonly h3indexPropertyName: string;

    constructor(h3indexPropertyName: string | undefined) {
        super();
        this.dataProjection = getProjection('EPSG:4326')
        this.h3indexPropertyName = h3indexPropertyName || "h3index";
        this.supportedMediaTypes = [
            "application/vnd.apache.arrow.file",
        ]
    }

    getType(): any {
        return ARRAY_BUFFER;
    }

    readProjection(source: any): Projection {
        return getProjection('EPSG:4326');
    }

    readGeometry(h3index: H3Index, opt_options?: ReadOptions | undefined): Geometry {
        let extRing = h3ToGeoBoundary(h3index, true)
        let geom = new Polygon([extRing],)
        return transformGeometryWithOptions(geom, false, opt_options)
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
                if (columnName == this.h3indexPropertyName) {
                    continue
                } else {
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

    setLayers(layers: any) {
    }
}
