//! Geospatial map demo built from GeoJSON through `berthacharts-geo`.

use std::sync::Arc;

use berthacharts_core::{ChartSize, ChartSpec};
use berthacharts_geo::{
    features_from_geojson_str_with_options, GeoJsonReadOptions, GeoMapOptions, GeoMapSpec,
    GeoProjection,
};
use leptos::prelude::*;

use crate::chart_canvas::{BuildChart, ChartCanvas};
use crate::chart_chrome::{DisplayControls, DisplayToggleButton};

const W: u32 = 620;
const H: u32 = 390;

#[component]
pub fn View() -> impl IntoView {
    let show_data_labels = RwSignal::new(true);
    let show_legend = RwSignal::new(true);

    let spec = Arc::new(demo_geo_spec());
    let summary = spec.summary();
    let bounds_label = summary.bounds.map_or_else(
        || "n/a".to_string(),
        |bounds| format!("{:.1} x {:.1}", bounds.lon_span(), bounds.lat_span()),
    );
    let value_range = match (summary.value_min, summary.value_max) {
        (Some(min), Some(max)) => format!("{min:.0}-{max:.0}"),
        _ => "n/a".to_string(),
    };
    let distance_label = format!("{:.0}", summary.total_length_km);
    let build_spec = spec.clone();

    let build: BuildChart = Arc::new(move |ws| {
        build_spec
            .build_chart(ws, ChartSize::new(W, H))
            .expect("demo geo spec should be valid")
    });

    view! {
        <section id="geospatial-map" class="example">
            <div class="example-head">
                <div>
                    <h2>"GeoJSON Choropleth"</h2>
                    <p>
                        "Projected polygons, multipolygons, routes, and sites render from GeoJSON with graticules, value-scaled symbols, labels, tooltips, and snap targets."
                    </p>
                </div>
                <div class="stat-strip">
                    <span><strong>{summary.features.to_string()}</strong>" features"</span>
                    <span><strong>{summary.groups.to_string()}</strong>" groups"</span>
                    <span><strong>{summary.polygons.to_string()}</strong>" areas"</span>
                    <span><strong>{value_range}</strong>" index"</span>
                    <span><strong>{distance_label}</strong>" km"</span>
                    <span><strong>{bounds_label}</strong>" deg"</span>
                </div>
            </div>
            <DisplayControls label="Map display options">
                <DisplayToggleButton label="Data labels" state=show_data_labels />
                <DisplayToggleButton label="Legend" state=show_legend />
            </DisplayControls>
            <div class=move || chart_stage_class(
                "chart-stage geo-stage",
                show_data_labels.get(),
                show_legend.get(),
            )>
                <ChartCanvas width={W} height={H} builder={build} />
            </div>
            <MapLibreGeoLab />
        </section>
    }
}

#[component]
fn MapLibreGeoLab() -> impl IntoView {
    view! {
        <div class="maplibre-lab">
            <div class="maplibre-lab-head">
                <div>
                    <h3>"MapLibre layer options"</h3>
                    <p>"Interactive vector map variants using the same district, route, and site data as the chart spec."</p>
                </div>
                <div class="maplibre-mode-controls" aria-label="MapLibre map options">
                    {MAPLIBRE_MODES
                        .iter()
                        .map(|mode| {
                            let class = if mode.id == "choropleth" {
                                "maplibre-option-button is-active"
                            } else {
                                "maplibre-option-button"
                            };
                            let pressed = if mode.id == "choropleth" { "true" } else { "false" };
                            view! {
                                <button
                                    type="button"
                                    class=class
                                    aria-pressed=pressed
                                    data-maplibre-target="geo-maplibre-demo"
                                    data-maplibre-mode=mode.id
                                >
                                    <strong>{mode.label}</strong>
                                    <span>{mode.detail}</span>
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </div>
            <div class="maplibre-shell">
                <div
                    id="geo-maplibre-demo"
                    class="maplibre-frame"
                    data-maplibre-map="geo-options"
                    data-maplibre-mode="choropleth"
                    role="region"
                    aria-label="MapLibre geo options demo"
                ></div>
            </div>
        </div>
    }
}

#[derive(Clone, Copy)]
struct MapLibreMode {
    id: &'static str,
    label: &'static str,
    detail: &'static str,
}

const MAPLIBRE_MODES: &[MapLibreMode] = &[
    MapLibreMode {
        id: "choropleth",
        label: "Choropleth",
        detail: "value fills",
    },
    MapLibreMode {
        id: "regions",
        label: "Regions",
        detail: "category fills",
    },
    MapLibreMode {
        id: "bubbles",
        label: "Bubbles",
        detail: "scaled sites",
    },
    MapLibreMode {
        id: "routes",
        label: "Routes",
        detail: "network focus",
    },
    MapLibreMode {
        id: "heat",
        label: "Heat",
        detail: "demand density",
    },
    MapLibreMode {
        id: "extrusion",
        label: "3D",
        detail: "index height",
    },
];

fn demo_geo_spec() -> GeoMapSpec {
    let options = GeoJsonReadOptions {
        label_property: Some("name".to_string()),
        value_property: Some("index".to_string()),
        group_property: Some("region".to_string()),
    };
    let features = features_from_geojson_str_with_options(DEMO_GEOJSON, &options)
        .expect("demo GeoJSON should parse");

    GeoMapSpec::new(features).with_options(GeoMapOptions {
        projection: GeoProjection::WebMercator,
        value_domain: Some((45.0, 92.0)),
        legend_title: "Access index".to_string(),
        graticule_lon_step: 0.08,
        graticule_lat_step: 0.08,
        point_radius_range: Some((4.0, 12.0)),
        line_width_range: Some((2.0, 6.0)),
        simplify_tolerance: 0.18,
        max_labels: 10,
        ..GeoMapOptions::default()
    })
}

fn chart_stage_class(base: &'static str, show_data_labels: bool, show_legend: bool) -> String {
    let mut class = String::from(base);
    if !show_data_labels {
        class.push_str(" hide-data-labels");
    }
    if !show_legend {
        class.push_str(" hide-legend");
    }
    class
}

const DEMO_GEOJSON: &str = r#"
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": { "name": "Harbor", "index": 74, "region": "Core" },
      "geometry": {
        "type": "Polygon",
        "coordinates": [[
          [-74.04, 40.69], [-73.94, 40.70], [-73.91, 40.77],
          [-73.98, 40.82], [-74.07, 40.78], [-74.04, 40.69]
        ], [
          [-74.00, 40.735], [-73.975, 40.745], [-73.985, 40.765],
          [-74.015, 40.760], [-74.00, 40.735]
        ]]
      }
    },
    {
      "type": "Feature",
      "properties": { "name": "North Ridge", "index": 58, "region": "Uptown" },
      "geometry": {
        "type": "Polygon",
        "coordinates": [[
          [-73.98, 40.82], [-73.87, 40.78], [-73.82, 40.86],
          [-73.90, 40.94], [-74.00, 40.91], [-73.98, 40.82]
        ]]
      }
    },
    {
      "type": "Feature",
      "properties": { "name": "East Flats", "index": 86, "region": "Growth" },
      "geometry": {
        "type": "Polygon",
        "coordinates": [[
          [-73.91, 40.69], [-73.79, 40.66], [-73.74, 40.72],
          [-73.82, 40.80], [-73.91, 40.77], [-73.91, 40.69]
        ]]
      }
    },
    {
      "type": "Feature",
      "properties": { "name": "South Bay", "index": 49, "region": "Outer" },
      "geometry": {
        "type": "MultiPolygon",
        "coordinates": [
          [[
            [-74.12, 40.55], [-73.98, 40.56], [-73.91, 40.64],
            [-74.00, 40.70], [-74.13, 40.66], [-74.12, 40.55]
          ]],
          [[
            [-73.91, 40.54], [-73.84, 40.55], [-73.82, 40.61],
            [-73.89, 40.63], [-73.93, 40.59], [-73.91, 40.54]
          ]]
        ]
      }
    },
    {
      "type": "Feature",
      "properties": { "name": "West Shelf", "index": 62, "region": "Outer" },
      "geometry": {
        "type": "Polygon",
        "coordinates": [[
          [-74.18, 40.70], [-74.07, 40.70], [-74.06, 40.79],
          [-74.16, 40.84], [-74.22, 40.78], [-74.18, 40.70]
        ]]
      }
    },
    {
      "type": "Feature",
      "properties": { "name": "Transit Spine", "index": 92, "region": "Route" },
      "geometry": {
        "type": "MultiLineString",
        "coordinates": [
          [
            [-74.10, 40.58], [-74.02, 40.66], [-73.96, 40.74],
            [-73.91, 40.84], [-73.87, 40.92]
          ],
          [
            [-74.15, 40.77], [-74.03, 40.76], [-73.89, 40.75], [-73.78, 40.72]
          ]
        ]
      }
    },
    {
      "type": "Feature",
      "properties": { "name": "Signal Nodes", "index": 67, "region": "Site" },
      "geometry": {
        "type": "MultiPoint",
        "coordinates": [
          [-73.88, 40.74], [-74.04, 40.80], [-73.84, 40.86]
        ]
      }
    }
  ]
}
"#;
