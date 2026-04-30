//! Three.js-backed 3D chart demos mounted by `three_charts.js`.

use leptos::prelude::*;

#[component]
pub fn View() -> impl IntoView {
    view! {
        <section id="three-d-charts" class="example three-d-charts">
            <div class="example-head">
                <div>
                    <h2>"3D Revenue Views"</h2>
                    <p>
                        "Interactive WebGL scenes combine monthly revenue composition, target bands, scenario terrain, and hover diagnostics."
                    </p>
                </div>
                <div class="stat-strip">
                    <span><strong>"$182k"</strong>" modeled ARR"</span>
                    <span><strong>"+18%"</strong>" expansion"</span>
                    <span><strong>"4.7x"</strong>" LTV:CAC"</span>
                    <span><strong>"6"</strong>" cohorts"</span>
                </div>
            </div>

            <div class="three-chart-grid">
                <article class="three-chart-panel">
                    <div class="three-chart-copy">
                        <div>
                            <h3>"Segment Stack"</h3>
                            <p>"Monthly MRR stack with target band, run-rate trend, and segment contribution on hover."</p>
                        </div>
                        <div class="three-panel-metric">
                            <strong>"$67k"</strong>
                            <span>"June MRR"</span>
                        </div>
                    </div>
                    <div
                        class="three-chart-canvas"
                        data-three-chart="bars"
                        role="img"
                        aria-label="3D column chart of monthly revenue by segment"
                    ></div>
                    <div class="three-chart-foot">
                        <span><i class="three-swatch three-swatch-base"></i>"Base"</span>
                        <span><i class="three-swatch three-swatch-expansion"></i>"Expansion"</span>
                        <span><i class="three-swatch three-swatch-new"></i>"New"</span>
                        <em>"Drag to rotate · hover for values"</em>
                    </div>
                </article>

                <article class="three-chart-panel">
                    <div class="three-chart-copy">
                        <div>
                            <h3>"Scenario Terrain"</h3>
                            <p>"Response surface for sales capacity, retention, and expansion sensitivity with scenario markers."</p>
                        </div>
                        <div class="three-panel-metric">
                            <strong>"+28%"</strong>
                            <span>"upside ridge"</span>
                        </div>
                    </div>
                    <div
                        class="three-chart-canvas"
                        data-three-chart="surface"
                        role="img"
                        aria-label="3D surface chart of revenue forecast scenarios"
                    ></div>
                    <div class="three-chart-foot">
                        <span><i class="three-swatch three-swatch-low"></i>"Constrained"</span>
                        <span><i class="three-swatch three-swatch-mid"></i>"Base"</span>
                        <span><i class="three-swatch three-swatch-high"></i>"Upside"</span>
                        <em>"Markers expose modeled ARR"</em>
                    </div>
                </article>
            </div>
        </section>
    }
}
