//! Gallery runtime and example catalog.
//!
//! The app shell should not need to know every chart module directly. This
//! module centralizes the gallery order, navigation labels, and runtime profile
//! so examples can adapt to local demos, hosted previews, and alternate data
//! fixtures without duplicating wiring.

use leptos::prelude::*;

use crate::examples;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataProfile {
    Retail,
    Growth,
    Operations,
}

impl DataProfile {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Retail => "Retail",
            Self::Growth => "Growth",
            Self::Operations => "Operations",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "retail" | "commerce" => Some(Self::Retail),
            "growth" | "saas" => Some(Self::Growth),
            "ops" | "operations" => Some(Self::Operations),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeEnvironment {
    Local,
    Hosted,
    Static,
}

impl RuntimeEnvironment {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Local => "Local",
            Self::Hosted => "Hosted",
            Self::Static => "Static",
        }
    }
}

#[derive(Clone, Debug)]
pub struct GalleryRuntime {
    pub data_profile: DataProfile,
    pub environment: RuntimeEnvironment,
}

impl GalleryRuntime {
    pub fn meta_label(&self) -> String {
        format!(
            "v0.0.1 · Leptos · WebGL2 · {} · {} data",
            self.environment.label(),
            self.data_profile.label()
        )
    }
}

pub fn runtime_context() -> GalleryRuntime {
    use_context::<GalleryRuntime>().unwrap_or_else(detect_runtime)
}

pub fn detect_runtime() -> GalleryRuntime {
    let (hostname, search) = browser_location();
    let data_profile = query_param(&search, "profile")
        .or_else(|| query_param(&search, "data"))
        .and_then(|value| DataProfile::from_slug(&value.to_ascii_lowercase()))
        .unwrap_or(DataProfile::Retail);

    GalleryRuntime {
        data_profile,
        environment: detect_environment(&hostname),
    }
}

fn detect_environment(hostname: &str) -> RuntimeEnvironment {
    match hostname {
        "" => RuntimeEnvironment::Static,
        "localhost" | "127.0.0.1" | "::1" => RuntimeEnvironment::Local,
        _ => RuntimeEnvironment::Hosted,
    }
}

fn query_param(search: &str, key: &str) -> Option<String> {
    search
        .trim_start_matches('?')
        .split('&')
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.split_once('='))
        .find_map(|(name, value)| (name == key).then(|| value.replace('+', " ")))
}

#[cfg(target_arch = "wasm32")]
fn browser_location() -> (String, String) {
    web_sys::window()
        .map(|window| {
            let location = window.location();
            let hostname = location.hostname().unwrap_or_default();
            let search = location.search().unwrap_or_default();
            (hostname, search)
        })
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn browser_location() -> (String, String) {
    (String::new(), String::new())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExampleKind {
    HelloRect,
    BarChart,
    CompMtdSales,
    UnifiedDataStudio,
    LineChart,
    GrowthModel,
    StateSpaceModels,
    StateSpaceFrameworks,
    PlanningWorkspace,
    ScatterPlot,
    GameTheory,
    AdvancedGames,
    MarkovChurn,
    WhittleIndex,
    ThompsonSampling,
    GeospatialMap,
    Grid,
    Sankey,
    Sunburst,
    ThreeD,
}

#[derive(Clone, Copy, Debug)]
pub struct ExampleDescriptor {
    pub kind: ExampleKind,
    pub id: &'static str,
    pub nav_label: &'static str,
}

impl ExampleDescriptor {
    pub const fn new(kind: ExampleKind, id: &'static str, nav_label: &'static str) -> Self {
        Self {
            kind,
            id,
            nav_label,
        }
    }

    pub fn href(self) -> String {
        format!("#{}", self.id)
    }
}

pub const EXAMPLES: &[ExampleDescriptor] = &[
    ExampleDescriptor::new(ExampleKind::HelloRect, "hello-rect", "Layers"),
    ExampleDescriptor::new(ExampleKind::BarChart, "bar-chart", "Revenue Bars"),
    ExampleDescriptor::new(ExampleKind::CompMtdSales, "comp-mtd-sales", "Comp MTD"),
    ExampleDescriptor::new(
        ExampleKind::UnifiedDataStudio,
        "unified-data-studio",
        "Unified Data",
    ),
    ExampleDescriptor::new(ExampleKind::LineChart, "line-chart", "Lines"),
    ExampleDescriptor::new(ExampleKind::GrowthModel, "growth-model", "Growth Model"),
    ExampleDescriptor::new(
        ExampleKind::StateSpaceModels,
        "state-space-models",
        "State Space",
    ),
    ExampleDescriptor::new(
        ExampleKind::StateSpaceFrameworks,
        "state-space-frameworks",
        "SSM Frameworks",
    ),
    ExampleDescriptor::new(
        ExampleKind::PlanningWorkspace,
        "planning-workspace",
        "Planning",
    ),
    ExampleDescriptor::new(ExampleKind::ScatterPlot, "scatter-plot", "Scatter"),
    ExampleDescriptor::new(ExampleKind::GameTheory, "game-theory", "Game Theory"),
    ExampleDescriptor::new(
        ExampleKind::AdvancedGames,
        "advanced-games",
        "Advanced Games",
    ),
    ExampleDescriptor::new(ExampleKind::MarkovChurn, "markov-churn", "Markov Churn"),
    ExampleDescriptor::new(ExampleKind::WhittleIndex, "whittle-index", "Whittle Index"),
    ExampleDescriptor::new(
        ExampleKind::ThompsonSampling,
        "thompson-sampling",
        "Thompson",
    ),
    ExampleDescriptor::new(ExampleKind::GeospatialMap, "geospatial-map", "Geo"),
    ExampleDescriptor::new(ExampleKind::Grid, "appointment-heatmap", "Appointments"),
    ExampleDescriptor::new(ExampleKind::Sankey, "sankey", "Sankey"),
    ExampleDescriptor::new(ExampleKind::Sunburst, "sunburst", "Sunburst"),
    ExampleDescriptor::new(ExampleKind::ThreeD, "three-d-charts", "3D"),
];

pub fn render_example(example: ExampleDescriptor) -> AnyView {
    match example.kind {
        ExampleKind::HelloRect => view! { <examples::hello_rect::View /> }.into_any(),
        ExampleKind::BarChart => view! { <examples::bar_chart::View /> }.into_any(),
        ExampleKind::CompMtdSales => view! { <examples::comp_mtd_sales::View /> }.into_any(),
        ExampleKind::UnifiedDataStudio => {
            view! { <examples::unified_data_studio::View /> }.into_any()
        }
        ExampleKind::LineChart => view! { <examples::line_chart::View /> }.into_any(),
        ExampleKind::GrowthModel => view! { <examples::growth_model::View /> }.into_any(),
        ExampleKind::StateSpaceModels => {
            view! { <examples::state_space_models::View /> }.into_any()
        }
        ExampleKind::StateSpaceFrameworks => {
            view! { <examples::state_space_frameworks::View /> }.into_any()
        }
        ExampleKind::PlanningWorkspace => {
            view! { <examples::planning_workspace::View /> }.into_any()
        }
        ExampleKind::ScatterPlot => view! { <examples::scatter_plot::View /> }.into_any(),
        ExampleKind::GameTheory => view! { <examples::game_theory::View /> }.into_any(),
        ExampleKind::AdvancedGames => view! { <examples::advanced_games::View /> }.into_any(),
        ExampleKind::MarkovChurn => view! { <examples::markov_churn::View /> }.into_any(),
        ExampleKind::WhittleIndex => view! { <examples::whittle_index::View /> }.into_any(),
        ExampleKind::ThompsonSampling => view! { <examples::thompson_sampling::View /> }.into_any(),
        ExampleKind::GeospatialMap => view! { <examples::geospatial_map::View /> }.into_any(),
        ExampleKind::Grid => view! { <examples::grid::View /> }.into_any(),
        ExampleKind::Sankey => view! { <examples::sankey::View /> }.into_any(),
        ExampleKind::Sunburst => view! { <examples::sunburst::View /> }.into_any(),
        ExampleKind::ThreeD => view! { <examples::three_d::View /> }.into_any(),
    }
}
