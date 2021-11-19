import 'ol/ol.css';
import Map from 'ol/Map';
import VectorLayer from "ol/layer/Vector";
import VectorSource from "ol/source/Vector";
import {GeoJSON} from "ol/format";
import View from 'ol/View';
import {Fill, Stroke, Style, Text} from "ol/style";
import VectorTileLayer from "ol/layer/VectorTile";
import VectorTileSource from 'ol/source/VectorTile'
import JsonLH3 from "./format/jsonlh3";
import {Feature} from "ol";
import {Geometry} from "ol/geom";
import {scaleLinear} from 'd3-scale'
import {getViewerConfig} from "./config";

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

const getView = () => {
    let view = new View({
        center: [0, 0],
        zoom: 1,
    });
    if (window.location.hash !== '') {
        // try to restore center, zoom-level and rotation from the URL
        const hash = window.location.hash.replace('#map=', '');
        const parts = hash.split('/');
        if (parts.length === 4) {
            view = new View({
                    zoom: parseFloat(parts[0]),
                    center: [parseFloat(parts[1]), parseFloat(parts[2])],
                    rotation: parseFloat(parts[3]),
                }
            )
        }
    }
    return view;
}


const cellStyleFn = () => {
    const stroke = new Stroke({
        color: '#333',
        width: 0.5,
    });
    const styleConfig = getViewerConfig().styleConfig;
    if (styleConfig) {
        /*
        const color = scaleLinear()
            .domain([-100, 0, +100])
            .range(["red", "white", "green"]);
         */
        const color = scaleLinear(styleConfig.valueRange, styleConfig.colorRange);
        return (feature: Feature<Geometry>): Style => {
            return new Style({
                fill: new Fill({
                    color: color(feature.get(styleConfig.propertyName)),
                }),
                stroke: stroke,
            });
        }
    } else {
        return (feature:Feature<Geometry>) => {
            return new Style({
                fill: new Fill({
                    color: 'green',
                }),
                stroke: stroke,
            });
        };
    }
}

const map = new Map({
    target: 'map',
    layers: [
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
        new VectorTileLayer({
                declutter: true,
                source: new VectorTileSource({
                        url: getViewerConfig().baseUrl + "/tiles/" + getViewerConfig().datasetName + '/{z}/{x}/{y}/jsonl',
                        format: new JsonLH3(getViewerConfig().h3indexPropertyName),
                    }
                ),
                style: cellStyleFn(),
            }
        ),
    ],
    view: getView(),
});


let shouldUpdate = true;
const view = map.getView();
const updatePermalink = function () {
    if (!shouldUpdate) {
        // do not update the URL when the view was changed in the 'popstate' handler
        shouldUpdate = true;
        return;
    }

    const center = view.getCenter();
    const hash =
        '#map=' +
        view.getZoom().toFixed(2) +
        '/' +
        center[0].toFixed(2) +
        '/' +
        center[1].toFixed(2) +
        '/' +
        view.getRotation();
    const state = {
        zoom: view.getZoom(),
        center: view.getCenter(),
        rotation: view.getRotation(),
    };
    window.history.pushState(state, 'map', hash);
};

map.on('moveend', updatePermalink);

// restore the view state when navigating through the history, see
// https://developer.mozilla.org/en-US/docs/Web/API/WindowEventHandlers/onpopstate
window.addEventListener('popstate', function (event) {
    if (event.state === null) {
        return;
    }
    map.getView().setCenter(event.state.center);
    map.getView().setZoom(event.state.zoom);
    map.getView().setRotation(event.state.rotation);
    shouldUpdate = false;
});