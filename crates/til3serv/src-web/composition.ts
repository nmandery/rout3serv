import {Layer} from "ol/layer";
import {Fill, Stroke, Style, Text} from "ol/style";
import {getViewerConfig} from "./config";
import VectorSource from "ol/source/Vector";
import VectorLayer from "ol/layer/Vector";
import {GeoJSON} from "ol/format";
import TileLayer from "ol/layer/Tile";
import {XYZ} from "ol/source";

const countryStyle = new Style({
    fill: new Fill({
        color: 'rgba(255, 255, 255, 0.6)',
    }),
    stroke: new Stroke({
        color: '#319FD3',
        width: 1,
    }),
    text: new Text({
        font: '12px Calibri,sans-serif',
        fill: new Fill({
            color: '#000',
        }),
        stroke: new Stroke({
            color: '#fff',
            width: 1,
        }),
    }),
});

export function layerComposition(mainLayer: Layer<any, any>, compositionName: string): Layer<any, any>[] {
    switch (compositionName) {
        case "eoc-baseoverlay": {
            return [
                mainLayer,
                new TileLayer({
                    source: new XYZ({
                        attributions: [
                            "Baseoverlay: Data © <a href=\"http://openstreetmap.org\">OpenStreetMap contributors</a> "
                            + "and <a href=\"https://geoservice.dlr.de/web/about#basemaps\">others</a>, "
                            + "Rendering © <a href=\"http://www.dlr.de/eoc\">DLR/EOC</a>"
                        ],
                        url: 'https://tiles.geoservice.dlr.de/service/tms/1.0.0/eoc%3Abaseoverlay@EPSG%3A3857@png/{z}/{x}/{y}.png?flipy=true',
                        maxZoom: 12,
                        crossOrigin: "anonymous",
                    }),
                }),
            ]
        }
        default: {
            if (compositionName != "builtin") {
                console.log("unknown layer composition '" + compositionName + "'. falling back to default");
            }
            return [
                new VectorLayer({
                    source: new VectorSource({
                        url: getViewerConfig().baseUrl + '/_ui/countries.geojson',
                        format: new GeoJSON(),
                    }),
                    style: function (feature) {
                        countryStyle.getText().setText(feature.get('name'));
                        return countryStyle;
                    },
                }),
                mainLayer
            ];
        }
    }
}
