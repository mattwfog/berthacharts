const MAPS = new WeakMap();

const AREA_GEOJSON = {
  type: "FeatureCollection",
  features: [
    {
      type: "Feature",
      properties: { name: "Harbor", index: 74, region: "Core" },
      geometry: {
        type: "Polygon",
        coordinates: [[
          [-74.04, 40.69], [-73.94, 40.70], [-73.91, 40.77],
          [-73.98, 40.82], [-74.07, 40.78], [-74.04, 40.69]
        ], [
          [-74.00, 40.735], [-73.975, 40.745], [-73.985, 40.765],
          [-74.015, 40.760], [-74.00, 40.735]
        ]]
      }
    },
    {
      type: "Feature",
      properties: { name: "North Ridge", index: 58, region: "Uptown" },
      geometry: {
        type: "Polygon",
        coordinates: [[
          [-73.98, 40.82], [-73.87, 40.78], [-73.82, 40.86],
          [-73.90, 40.94], [-74.00, 40.91], [-73.98, 40.82]
        ]]
      }
    },
    {
      type: "Feature",
      properties: { name: "East Flats", index: 86, region: "Growth" },
      geometry: {
        type: "Polygon",
        coordinates: [[
          [-73.91, 40.69], [-73.79, 40.66], [-73.74, 40.72],
          [-73.82, 40.80], [-73.91, 40.77], [-73.91, 40.69]
        ]]
      }
    },
    {
      type: "Feature",
      properties: { name: "South Bay", index: 49, region: "Outer" },
      geometry: {
        type: "MultiPolygon",
        coordinates: [
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
      type: "Feature",
      properties: { name: "West Shelf", index: 62, region: "Outer" },
      geometry: {
        type: "Polygon",
        coordinates: [[
          [-74.18, 40.70], [-74.07, 40.70], [-74.06, 40.79],
          [-74.16, 40.84], [-74.22, 40.78], [-74.18, 40.70]
        ]]
      }
    }
  ]
};

const ROUTE_GEOJSON = {
  type: "FeatureCollection",
  features: [
    {
      type: "Feature",
      properties: { name: "Transit Spine", index: 92, region: "Route" },
      geometry: {
        type: "MultiLineString",
        coordinates: [
          [
            [-74.10, 40.58], [-74.02, 40.66], [-73.96, 40.74],
            [-73.91, 40.84], [-73.87, 40.92]
          ],
          [
            [-74.15, 40.77], [-74.03, 40.76], [-73.89, 40.75], [-73.78, 40.72]
          ]
        ]
      }
    }
  ]
};

const SITE_GEOJSON = {
  type: "FeatureCollection",
  features: [
    pointFeature("Signal A", -73.88, 40.74, 67, "Site"),
    pointFeature("Signal B", -74.04, 40.80, 67, "Site"),
    pointFeature("Signal C", -73.84, 40.86, 67, "Site")
  ]
};

const DEMAND_GEOJSON = {
  type: "FeatureCollection",
  features: [
    pointFeature("Harbor", -73.985, 40.752, 74, "Core"),
    pointFeature("North Ridge", -73.915, 40.866, 58, "Uptown"),
    pointFeature("East Flats", -73.842, 40.727, 86, "Growth"),
    pointFeature("South Bay", -74.004, 40.618, 49, "Outer"),
    pointFeature("West Shelf", -74.144, 40.764, 62, "Outer"),
    pointFeature("Signal A", -73.88, 40.74, 67, "Site"),
    pointFeature("Signal B", -74.04, 40.80, 67, "Site"),
    pointFeature("Signal C", -73.84, 40.86, 67, "Site")
  ]
};

const ALL_BOUNDS = [
  [-74.23, 40.53],
  [-73.73, 40.95]
];

const VALUE_RAMP = [
  "interpolate", ["linear"], ["get", "index"],
  45, "#dceaf0",
  62, "#90c7c1",
  78, "#2f8f9c",
  92, "#123e58"
];

const REGION_RAMP = [
  "match", ["get", "region"],
  "Core", "#2f6fbb",
  "Uptown", "#7b61a9",
  "Growth", "#159b7a",
  "Outer", "#d8694e",
  "Route", "#26374d",
  "Site", "#c89d20",
  "#8a95a8"
];

function pointFeature(name, lon, lat, index, region) {
  return {
    type: "Feature",
    properties: { name, index, region },
    geometry: { type: "Point", coordinates: [lon, lat] }
  };
}

function emptyStyle() {
  return {
    version: 8,
    glyphs: "https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf",
    sources: {
      "carto-light": {
        type: "raster",
        tiles: [
          "https://a.basemaps.cartocdn.com/light_all/{z}/{x}/{y}.png",
          "https://b.basemaps.cartocdn.com/light_all/{z}/{x}/{y}.png",
          "https://c.basemaps.cartocdn.com/light_all/{z}/{x}/{y}.png",
          "https://d.basemaps.cartocdn.com/light_all/{z}/{x}/{y}.png"
        ],
        tileSize: 256,
        attribution: "&copy; OpenStreetMap contributors &copy; CARTO"
      }
    },
    layers: [
      {
        id: "geo-background",
        type: "background",
        paint: { "background-color": "#e8eef3" }
      },
      {
        id: "geo-basemap",
        type: "raster",
        source: "carto-light",
        paint: {
          "raster-opacity": 0.86,
          "raster-saturation": -0.45,
          "raster-contrast": -0.08
        }
      }
    ]
  };
}

function initializeMaps(root = document) {
  if (!window.maplibregl) {
    markMissingMapLibre(root);
    return;
  }

  if (root.matches?.("[data-maplibre-map]") && !MAPS.has(root)) {
    initializeMap(root);
  }

  root.querySelectorAll("[data-maplibre-map]").forEach((container) => {
    if (!MAPS.has(container)) {
      initializeMap(container);
    }
  });
}

function initializeMap(container) {
  container.textContent = "";
  const map = new maplibregl.Map({
    container,
    style: emptyStyle(),
    center: [-73.965, 40.745],
    zoom: 9.55,
    pitch: 0,
    bearing: 0,
    attributionControl: false,
    antialias: true
  });

  const state = {
    map,
    loaded: false,
    mode: container.dataset.maplibreMode || "choropleth"
  };
  MAPS.set(container, state);

  map.addControl(new maplibregl.NavigationControl({ visualizePitch: true }), "top-right");
  map.addControl(new maplibregl.AttributionControl({ compact: true }), "bottom-right");

  map.on("load", () => {
    addSources(map);
    addLayers(map);
    state.loaded = true;
    fitDemoBounds(map, 0);
    applyMode(container, state.mode);
  });

  map.on("error", (event) => {
    console.warn("MapLibre geo demo error", event.error || event);
  });

  if ("ResizeObserver" in window) {
    const observer = new ResizeObserver(() => map.resize());
    observer.observe(container);
  }
}

function addSources(map) {
  map.addSource("geo-areas", { type: "geojson", data: AREA_GEOJSON });
  map.addSource("geo-routes", { type: "geojson", data: ROUTE_GEOJSON });
  map.addSource("geo-sites", { type: "geojson", data: SITE_GEOJSON });
  map.addSource("geo-demand", { type: "geojson", data: DEMAND_GEOJSON });
}

function addLayers(map) {
  map.addLayer({
    id: "geo-area-fill",
    type: "fill",
    source: "geo-areas",
    paint: {
      "fill-color": VALUE_RAMP,
      "fill-opacity": 0.82
    }
  });
  map.addLayer({
    id: "geo-area-outline",
    type: "line",
    source: "geo-areas",
    paint: {
      "line-color": "#ffffff",
      "line-opacity": 0.95,
      "line-width": 1.6
    }
  });
  map.addLayer({
    id: "geo-route-line",
    type: "line",
    source: "geo-routes",
    layout: { "line-cap": "round", "line-join": "round" },
    paint: {
      "line-color": "#25364d",
      "line-opacity": 0.9,
      "line-width": 3.2
    }
  });
  map.addLayer({
    id: "geo-site-halo",
    type: "circle",
    source: "geo-sites",
    paint: {
      "circle-color": "#ffffff",
      "circle-opacity": 0.72,
      "circle-radius": 10,
      "circle-stroke-color": "#cfd8e3",
      "circle-stroke-width": 1
    }
  });
  map.addLayer({
    id: "geo-site-circle",
    type: "circle",
    source: "geo-sites",
    paint: {
      "circle-color": "#c89d20",
      "circle-opacity": 0.95,
      "circle-radius": 5.5,
      "circle-stroke-color": "#ffffff",
      "circle-stroke-width": 1.5
    }
  });
  map.addLayer({
    id: "geo-demand-heat",
    type: "heatmap",
    source: "geo-demand",
    paint: {
      "heatmap-weight": ["interpolate", ["linear"], ["get", "index"], 45, 0.25, 92, 1],
      "heatmap-intensity": 1.25,
      "heatmap-radius": ["interpolate", ["linear"], ["zoom"], 8, 28, 11, 54],
      "heatmap-opacity": 0,
      "heatmap-color": [
        "interpolate", ["linear"], ["heatmap-density"],
        0, "rgba(19, 54, 76, 0)",
        0.22, "#9bd4ce",
        0.5, "#34a08f",
        0.78, "#f0b75d",
        1, "#d65d4d"
      ]
    }
  });
  map.addLayer({
    id: "geo-area-extrusion",
    type: "fill-extrusion",
    source: "geo-areas",
    paint: {
      "fill-extrusion-color": VALUE_RAMP,
      "fill-extrusion-height": ["interpolate", ["linear"], ["get", "index"], 45, 220, 92, 2100],
      "fill-extrusion-base": 0,
      "fill-extrusion-opacity": 0
    }
  });
  map.addLayer({
    id: "geo-area-label",
    type: "symbol",
    source: "geo-demand",
    layout: {
      "text-field": ["get", "name"],
      "text-font": ["Noto Sans Regular"],
      "text-size": 11,
      "text-offset": [0, 1.35],
      "text-anchor": "top"
    },
    paint: {
      "text-color": "#263244",
      "text-halo-color": "rgba(255, 255, 255, 0.86)",
      "text-halo-width": 1.2,
      "text-opacity": 0.92
    }
  });
}

function applyMode(container, mode) {
  const state = MAPS.get(container);
  if (!state) return;
  state.mode = mode;
  container.dataset.maplibreMode = mode;
  if (!state.loaded) return;

  const { map } = state;
  show(map, ["geo-area-fill", "geo-area-outline", "geo-route-line", "geo-site-halo", "geo-site-circle", "geo-area-label"]);
  hide(map, ["geo-demand-heat", "geo-area-extrusion"]);
  setPaint(map, "geo-area-fill", {
    "fill-color": VALUE_RAMP,
    "fill-opacity": 0.82
  });
  setPaint(map, "geo-area-outline", {
    "line-color": "#ffffff",
    "line-opacity": 0.95,
    "line-width": 1.6
  });
  setPaint(map, "geo-route-line", {
    "line-color": "#25364d",
    "line-opacity": 0.9,
    "line-width": 3.2
  });
  setPaint(map, "geo-site-circle", {
    "circle-color": "#c89d20",
    "circle-opacity": 0.95,
    "circle-radius": 5.5
  });
  setPaint(map, "geo-site-halo", {
    "circle-opacity": 0.72,
    "circle-radius": 10
  });
  setPaint(map, "geo-area-label", { "text-opacity": 0.92 });

  if (mode === "regions") {
    setPaint(map, "geo-area-fill", {
      "fill-color": REGION_RAMP,
      "fill-opacity": 0.78
    });
    setPaint(map, "geo-route-line", {
      "line-color": "#2b405c",
      "line-opacity": 0.62,
      "line-width": 2.4
    });
    easeFlat(map);
  } else if (mode === "bubbles") {
    setPaint(map, "geo-area-fill", {
      "fill-color": "#dfe7ef",
      "fill-opacity": 0.58
    });
    setPaint(map, "geo-site-circle", {
      "circle-color": VALUE_RAMP,
      "circle-opacity": 0.9,
      "circle-radius": ["interpolate", ["linear"], ["get", "index"], 45, 7, 92, 18]
    });
    setPaint(map, "geo-site-halo", {
      "circle-opacity": 0.86,
      "circle-radius": ["interpolate", ["linear"], ["get", "index"], 45, 12, 92, 24]
    });
    show(map, ["geo-demand-heat"]);
    setPaint(map, "geo-demand-heat", { "heatmap-opacity": 0.2 });
    easeFlat(map);
  } else if (mode === "routes") {
    setPaint(map, "geo-area-fill", {
      "fill-color": "#e4ebf1",
      "fill-opacity": 0.46
    });
    setPaint(map, "geo-area-outline", {
      "line-color": "#b7c2cf",
      "line-opacity": 0.86,
      "line-width": 1.1
    });
    setPaint(map, "geo-route-line", {
      "line-color": "#111d2c",
      "line-opacity": 0.98,
      "line-width": ["interpolate", ["linear"], ["zoom"], 8, 4, 11, 9]
    });
    setPaint(map, "geo-site-circle", {
      "circle-color": "#df604d",
      "circle-opacity": 0.96,
      "circle-radius": 7
    });
    easeFlat(map);
  } else if (mode === "heat") {
    setPaint(map, "geo-area-fill", {
      "fill-color": "#eef3f7",
      "fill-opacity": 0.42
    });
    setPaint(map, "geo-route-line", {
      "line-color": "#526176",
      "line-opacity": 0.44,
      "line-width": 2
    });
    setPaint(map, "geo-site-circle", {
      "circle-color": "#ffffff",
      "circle-opacity": 0.8,
      "circle-radius": 4.5
    });
    setPaint(map, "geo-area-label", { "text-opacity": 0.72 });
    show(map, ["geo-demand-heat"]);
    setPaint(map, "geo-demand-heat", { "heatmap-opacity": 0.78 });
    easeFlat(map);
  } else if (mode === "extrusion") {
    hide(map, ["geo-area-fill"]);
    show(map, ["geo-area-extrusion"]);
    setPaint(map, "geo-area-extrusion", {
      "fill-extrusion-opacity": 0.78,
      "fill-extrusion-color": VALUE_RAMP,
      "fill-extrusion-height": ["interpolate", ["linear"], ["get", "index"], 45, 220, 92, 2100]
    });
    setPaint(map, "geo-route-line", {
      "line-color": "#172234",
      "line-opacity": 0.78,
      "line-width": 2.6
    });
    map.easeTo({ pitch: 56, bearing: -24, zoom: 9.45, duration: 650 });
  } else {
    easeFlat(map);
  }
}

function setPaint(map, layerId, paint) {
  if (!map.getLayer(layerId)) return;
  for (const [property, value] of Object.entries(paint)) {
    map.setPaintProperty(layerId, property, value);
  }
}

function show(map, layerIds) {
  layerIds.forEach((layerId) => {
    if (map.getLayer(layerId)) map.setLayoutProperty(layerId, "visibility", "visible");
  });
}

function hide(map, layerIds) {
  layerIds.forEach((layerId) => {
    if (map.getLayer(layerId)) map.setLayoutProperty(layerId, "visibility", "none");
  });
}

function easeFlat(map) {
  map.easeTo({ pitch: 0, bearing: 0, zoom: 9.55, duration: 500 });
}

function fitDemoBounds(map, duration = 500) {
  map.fitBounds(ALL_BOUNDS, {
    padding: { top: 36, right: 38, bottom: 36, left: 38 },
    duration
  });
}

function markMissingMapLibre(root) {
  root.querySelectorAll("[data-maplibre-map]").forEach((container) => {
    if (container.dataset.maplibreMissing === "true") return;
    container.dataset.maplibreMissing = "true";
    container.innerHTML = "<div class=\"maplibre-status\">MapLibre GL JS did not load.</div>";
  });
}

function bindModeControls(root = document) {
  root.addEventListener("click", (event) => {
    const button = event.target.closest("[data-maplibre-target][data-maplibre-mode]");
    if (!button) return;
    const target = document.getElementById(button.dataset.maplibreTarget);
    if (!target) return;
    const mode = button.dataset.maplibreMode;
    applyMode(target, mode);
    document
      .querySelectorAll(`[data-maplibre-target="${button.dataset.maplibreTarget}"]`)
      .forEach((item) => {
        const selected = item.dataset.maplibreMode === mode;
        item.classList.toggle("is-active", selected);
        item.setAttribute("aria-pressed", selected ? "true" : "false");
      });
  });
}

bindModeControls();

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", () => initializeMaps());
} else {
  initializeMaps();
}

const observer = new MutationObserver((mutations) => {
  for (const mutation of mutations) {
    mutation.addedNodes.forEach((node) => {
      if (node.nodeType === Node.ELEMENT_NODE) {
        initializeMaps(node);
      }
    });
  }
});

observer.observe(document.documentElement, { childList: true, subtree: true });
