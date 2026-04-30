import * as THREE from "./vendor/three.module.min.js";

const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];
const segmentSeries = [
  { name: "Base", color: 0x3577d4, values: [18, 19, 21, 22, 24, 27] },
  { name: "Expansion", color: 0x139b80, values: [6, 8, 9, 11, 12, 15] },
  { name: "New", color: 0xdf604d, values: [12, 14, 18, 16, 21, 25] },
];

const surfaceRows = 13;
const surfaceCols = 15;
const surfaceXLabels = ["Capacity", "Execution"];
const surfaceZLabels = ["Retention", "Expansion"];
const preferredCamera = {
  bars: { position: new THREE.Vector3(6.8, 5.1, 8.4), target: new THREE.Vector3(0.1, 1.3, 0.1) },
  surface: { position: new THREE.Vector3(7.2, 5.0, 7.6), target: new THREE.Vector3(0, 1.2, 0) },
};

class ThreeChart {
  constructor(container, kind) {
    this.container = container;
    this.kind = kind;
    this.scene = new THREE.Scene();
    this.scene.fog = new THREE.Fog(0xf7f9fc, 12, 26);
    this.camera = new THREE.PerspectiveCamera(38, 1, 0.1, 120);
    this.camera.position.copy(preferredCamera[kind].position);
    this.camera.lookAt(preferredCamera[kind].target);
    this.renderer = new THREE.WebGLRenderer({
      antialias: true,
      alpha: false,
      preserveDrawingBuffer: true,
    });
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.08;
    this.renderer.setClearColor(0xf8fafd, 1);
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    this.container.appendChild(this.renderer.domElement);

    this.tooltip = document.createElement("div");
    this.tooltip.className = "three-chart-tooltip";
    this.container.appendChild(this.tooltip);

    this.root = new THREE.Group();
    this.scene.add(this.root);
    this.raycaster = new THREE.Raycaster();
    this.pointerNdc = new THREE.Vector2();
    this.interactive = [];
    this.hovered = null;
    this.targetRotation = { x: kind === "surface" ? -0.14 : -0.18, y: kind === "surface" ? -0.46 : -0.52 };
    this.rotation = { ...this.targetRotation };
    this.pointer = { active: false, x: 0, y: 0, moved: false };

    this.addLights();
    this.addFloor();

    if (kind === "surface") {
      buildSurfaceChart(this.root, this.interactive);
      this.container.dataset.summary = "Upside ridge +28% over base";
    } else {
      buildStackedBars(this.root, this.interactive);
      this.container.dataset.summary = "June run-rate: $67k";
    }

    this.resizeObserver = new ResizeObserver(() => this.resize());
    this.resizeObserver.observe(this.container);
    this.bindPointerEvents();
    this.resize();
    this.animate();
  }

  addLights() {
    this.scene.add(new THREE.HemisphereLight(0xffffff, 0xd5dde8, 1.65));

    const key = new THREE.DirectionalLight(0xffffff, 2.7);
    key.position.set(5, 8, 6);
    key.castShadow = true;
    key.shadow.mapSize.set(1536, 1536);
    key.shadow.camera.near = 1;
    key.shadow.camera.far = 24;
    key.shadow.camera.left = -8;
    key.shadow.camera.right = 8;
    key.shadow.camera.top = 8;
    key.shadow.camera.bottom = -8;
    this.scene.add(key);

    const rim = new THREE.DirectionalLight(0x9ccfff, 1.35);
    rim.position.set(-6, 4, -5);
    this.scene.add(rim);
  }

  addFloor() {
    const floor = new THREE.Mesh(
      new THREE.PlaneGeometry(13.5, 9.2),
      new THREE.MeshStandardMaterial({
        color: 0xf8fafd,
        roughness: 0.86,
        metalness: 0.01,
      }),
    );
    floor.rotation.x = -Math.PI / 2;
    floor.position.y = -0.05;
    floor.receiveShadow = true;
    this.scene.add(floor);

    const grid = new THREE.GridHelper(13.5, 13, 0xb7c4d4, 0xe1e7ef);
    grid.position.y = 0.004;
    this.scene.add(grid);
  }

  bindPointerEvents() {
    this.container.addEventListener("pointerdown", (event) => {
      this.pointer.active = true;
      this.pointer.moved = false;
      this.pointer.x = event.clientX;
      this.pointer.y = event.clientY;
      this.container.setPointerCapture(event.pointerId);
    });

    this.container.addEventListener("pointermove", (event) => {
      this.updatePointer(event);
      if (!this.pointer.active) {
        this.pick(event);
        return;
      }
      const dx = event.clientX - this.pointer.x;
      const dy = event.clientY - this.pointer.y;
      this.pointer.x = event.clientX;
      this.pointer.y = event.clientY;
      this.pointer.moved ||= Math.abs(dx) + Math.abs(dy) > 2;
      this.targetRotation.y += dx * 0.008;
      this.targetRotation.x = clamp(this.targetRotation.x + dy * 0.005, -0.72, 0.16);
      this.hideTooltip();
    });

    this.container.addEventListener("pointerup", (event) => {
      this.pointer.active = false;
      if (!this.pointer.moved) this.pick(event);
      if (this.container.hasPointerCapture(event.pointerId)) {
        this.container.releasePointerCapture(event.pointerId);
      }
    });

    this.container.addEventListener("pointercancel", () => {
      this.pointer.active = false;
      this.hideTooltip();
    });

    this.container.addEventListener("mouseleave", () => {
      this.pointer.active = false;
      this.setHovered(null);
      this.hideTooltip();
    });
  }

  updatePointer(event) {
    const rect = this.renderer.domElement.getBoundingClientRect();
    this.pointerNdc.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
    this.pointerNdc.y = -(((event.clientY - rect.top) / rect.height) * 2 - 1);
  }

  pick(event) {
    this.raycaster.setFromCamera(this.pointerNdc, this.camera);
    const [hit] = this.raycaster.intersectObjects(this.interactive, false);
    const object = hit?.object ?? null;
    this.setHovered(object);
    if (object?.userData?.label) {
      this.showTooltip(event, object.userData);
    } else {
      this.hideTooltip();
    }
  }

  setHovered(object) {
    if (this.hovered === object) return;
    if (this.hovered?.userData?.baseScale) {
      this.hovered.scale.copy(this.hovered.userData.baseScale);
    }
    if (this.hovered?.material?.emissive) {
      this.hovered.material.emissive.setHex(0x000000);
    }
    this.hovered = object;
    if (object?.userData?.baseScale) {
      object.scale.set(
        object.userData.baseScale.x * 1.04,
        object.userData.baseScale.y * 1.04,
        object.userData.baseScale.z * 1.04,
      );
    }
    if (object?.material?.emissive) {
      object.material.emissive.setHex(0x182234);
      object.material.emissiveIntensity = 0.08;
    }
  }

  showTooltip(event, data) {
    this.tooltip.innerHTML = `
      <strong>${data.label}</strong>
      <span>${data.value}</span>
      <em>${data.detail}</em>
    `;
    const rect = this.container.getBoundingClientRect();
    const left = clamp(event.clientX - rect.left + 12, 10, rect.width - 188);
    const top = clamp(event.clientY - rect.top + 12, 10, rect.height - 92);
    this.tooltip.style.transform = `translate(${left}px, ${top}px)`;
    this.tooltip.classList.add("is-visible");
  }

  hideTooltip() {
    this.tooltip.classList.remove("is-visible");
  }

  resize() {
    const { width, height } = this.container.getBoundingClientRect();
    const safeWidth = Math.max(280, Math.floor(width));
    const safeHeight = Math.max(260, Math.floor(height));
    this.renderer.setSize(safeWidth, safeHeight, false);
    this.camera.aspect = safeWidth / safeHeight;
    this.camera.fov = safeWidth < 420 ? 44 : 38;
    this.camera.position.copy(preferredCamera[this.kind].position);
    if (safeWidth < 420) {
      this.camera.position.multiplyScalar(this.kind === "surface" ? 1.14 : 1.2);
    }
    this.camera.updateProjectionMatrix();
    this.camera.lookAt(preferredCamera[this.kind].target);
  }

  animate() {
    this.frame = requestAnimationFrame(() => this.animate());
    if (!this.pointer.active) {
      this.targetRotation.y += this.kind === "surface" ? 0.001 : 0.0008;
    }
    this.rotation.x += (this.targetRotation.x - this.rotation.x) * 0.08;
    this.rotation.y += (this.targetRotation.y - this.rotation.y) * 0.08;
    this.root.rotation.set(this.rotation.x, this.rotation.y, 0);
    this.renderer.render(this.scene, this.camera);
  }

  destroy() {
    cancelAnimationFrame(this.frame);
    this.resizeObserver.disconnect();
    this.renderer.dispose();
    this.container.textContent = "";
  }
}

function buildStackedBars(root, interactive) {
  const group = new THREE.Group();
  group.position.set(-2.75, 0, -0.65);
  root.add(group);

  const totals = months.map((_, monthIndex) =>
    segmentSeries.reduce((sum, segment) => sum + segment.values[monthIndex], 0),
  );
  const maxTotal = Math.max(...totals);
  const monthSpacing = 1.08;
  const columnWidth = 0.56;
  const depth = 0.82;

  months.forEach((month, monthIndex) => {
    let y = 0;
    segmentSeries.forEach((segment) => {
      const value = segment.values[monthIndex];
      const height = (value / maxTotal) * 4.2;
      const material = new THREE.MeshStandardMaterial({
        color: segment.color,
        roughness: 0.5,
        metalness: 0.07,
      });
      const bar = new THREE.Mesh(new THREE.BoxGeometry(columnWidth, height, depth), material);
      bar.position.set(monthIndex * monthSpacing, y + height / 2, 0);
      bar.castShadow = true;
      bar.receiveShadow = true;
      bar.userData = {
        baseScale: bar.scale.clone(),
        label: `${month} ${segment.name}`,
        value: `$${value}k`,
        detail: `Stack contribution, ${Math.round((value / totals[monthIndex]) * 100)}% of month`,
      };
      interactive.push(bar);
      group.add(bar);
      y += height;
    });

    const totalLabel = makeLabel(`$${totals[monthIndex]}k`, "#182234", "800 24px system-ui", 220, 72);
    totalLabel.position.set(monthIndex * monthSpacing, y + 0.34, 0);
    totalLabel.scale.set(0.52, 0.18, 1);
    group.add(totalLabel);

    const monthLabel = makeLabel(month, "#657185", "800 22px system-ui", 180, 64);
    monthLabel.position.set(monthIndex * monthSpacing, 0.08, 1.45);
    monthLabel.scale.set(0.42, 0.16, 1);
    group.add(monthLabel);
  });

  addTargetBand(group, (58 / maxTotal) * 4.2, months.length * monthSpacing - 0.6);
  addTrendLine(group, totals, maxTotal, monthSpacing);
  addAxis(group, new THREE.Vector3(-0.45, 0, -0.58), new THREE.Vector3(5.95, 0, -0.58), "Months");
  addAxis(group, new THREE.Vector3(-0.45, 0, -0.58), new THREE.Vector3(-0.45, 4.85, -0.58), "MRR");
  addAxis(group, new THREE.Vector3(-0.45, 0, -0.58), new THREE.Vector3(-0.45, 0, 1.15), "Stack");

  const legend = makeLegend(segmentSeries);
  legend.position.set(2.7, 4.78, 1.18);
  group.add(legend);
}

function buildSurfaceChart(root, interactive) {
  const { geometry, values } = surfaceGeometry();
  const material = new THREE.MeshStandardMaterial({
    roughness: 0.58,
    metalness: 0.035,
    side: THREE.DoubleSide,
    vertexColors: true,
  });
  const surface = new THREE.Mesh(geometry, material);
  surface.castShadow = true;
  surface.receiveShadow = true;
  surface.userData = {
    baseScale: surface.scale.clone(),
    label: "Forecast terrain",
    value: "Base to upside envelope",
    detail: "Drag to rotate; red ridge shows strongest combined scenario",
  };
  interactive.push(surface);
  root.add(surface);

  const wireframe = new THREE.LineSegments(
    new THREE.WireframeGeometry(geometry),
    new THREE.LineBasicMaterial({ color: 0xffffff, transparent: true, opacity: 0.38 }),
  );
  root.add(wireframe);

  const shadow = new THREE.Mesh(
    projectedSurfaceGeometry(values),
    new THREE.MeshBasicMaterial({ color: 0x8291a6, transparent: true, opacity: 0.16, side: THREE.DoubleSide }),
  );
  shadow.position.y = 0.014;
  root.add(shadow);

  addContourLines(root, values);
  addScenarioMarkers(root, interactive, values);
  addAxis(root, new THREE.Vector3(-3.35, 0, -2.45), new THREE.Vector3(3.35, 0, -2.45), surfaceXLabels.join(" -> "));
  addAxis(root, new THREE.Vector3(-3.35, 0, -2.45), new THREE.Vector3(-3.35, 3.75, -2.45), "ARR");
  addAxis(root, new THREE.Vector3(-3.35, 0, -2.45), new THREE.Vector3(-3.35, 0, 2.45), surfaceZLabels.join(" -> "));

  const upside = makeLabel("Upside ridge", "#182234", "800 26px system-ui", 260, 80);
  upside.position.set(2.9, 3.15, -2.2);
  upside.scale.set(0.76, 0.24, 1);
  root.add(upside);

  const base = makeLabel("Base case", "#657185", "800 23px system-ui", 220, 70);
  base.position.set(0.15, 1.85, 0.3);
  base.scale.set(0.58, 0.2, 1);
  root.add(base);
}

function addTargetBand(group, y, width) {
  const band = new THREE.Mesh(
    new THREE.BoxGeometry(width, 0.035, 1.58),
    new THREE.MeshBasicMaterial({ color: 0x182234, transparent: true, opacity: 0.15 }),
  );
  band.position.set(width / 2 - 0.54, y, 0);
  group.add(band);

  const label = makeLabel("target $58k", "#182234", "800 20px system-ui", 220, 64);
  label.position.set(width - 0.8, y + 0.18, -0.95);
  label.scale.set(0.48, 0.16, 1);
  group.add(label);
}

function addTrendLine(group, totals, maxTotal, spacing) {
  const points = totals.map((total, index) =>
    new THREE.Vector3(index * spacing, (total / maxTotal) * 4.2 + 0.08, -0.72),
  );
  const line = new THREE.Line(
    new THREE.BufferGeometry().setFromPoints(points),
    new THREE.LineBasicMaterial({ color: 0x182234, linewidth: 2 }),
  );
  group.add(line);

  points.forEach((point) => {
    const marker = new THREE.Mesh(
      new THREE.SphereGeometry(0.075, 16, 16),
      new THREE.MeshStandardMaterial({ color: 0x182234, roughness: 0.42 }),
    );
    marker.position.copy(point);
    marker.castShadow = true;
    group.add(marker);
  });
}

function surfaceGeometry() {
  const geometry = new THREE.BufferGeometry();
  const positions = [];
  const colors = [];
  const indices = [];
  const values = [];
  const colorLow = new THREE.Color(0x3577d4);
  const colorMid = new THREE.Color(0x139b80);
  const colorHigh = new THREE.Color(0xdf604d);
  const width = 6.4;
  const depth = 4.7;

  for (let row = 0; row < surfaceRows; row += 1) {
    values[row] = [];
    for (let col = 0; col < surfaceCols; col += 1) {
      const xRatio = col / (surfaceCols - 1);
      const zRatio = row / (surfaceRows - 1);
      const acquisition = smoothstep(xRatio);
      const retention = smoothstep(1 - zRatio);
      const ridge = Math.exp(-((xRatio - 0.78) ** 2 + (zRatio - 0.22) ** 2) / 0.09) * 0.62;
      const valley = Math.exp(-((xRatio - 0.28) ** 2 + (zRatio - 0.68) ** 2) / 0.05) * 0.42;
      const wave = Math.sin(xRatio * Math.PI * 2.1) * 0.13 + Math.cos(zRatio * Math.PI * 1.7) * 0.11;
      const value = 0.35 + acquisition * 1.25 + retention * 0.9 + ridge - valley + wave;
      values[row][col] = value;
      positions.push(xRatio * width - width / 2, value, zRatio * depth - depth / 2);

      const mix = clamp(value / 3.0, 0, 1);
      const color = mix < 0.5
        ? colorLow.clone().lerp(colorMid, mix / 0.5)
        : colorMid.clone().lerp(colorHigh, (mix - 0.5) / 0.5);
      colors.push(color.r, color.g, color.b);
    }
  }

  for (let row = 0; row < surfaceRows - 1; row += 1) {
    for (let col = 0; col < surfaceCols - 1; col += 1) {
      const a = row * surfaceCols + col;
      const b = a + 1;
      const c = a + surfaceCols;
      const d = c + 1;
      indices.push(a, c, b, b, c, d);
    }
  }

  geometry.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
  geometry.setAttribute("color", new THREE.Float32BufferAttribute(colors, 3));
  geometry.setIndex(indices);
  geometry.computeVertexNormals();
  return { geometry, values };
}

function projectedSurfaceGeometry(values) {
  const geometry = new THREE.BufferGeometry();
  const positions = [];
  const indices = [];
  const width = 6.4;
  const depth = 4.7;
  for (let row = 0; row < surfaceRows; row += 1) {
    for (let col = 0; col < surfaceCols; col += 1) {
      const xRatio = col / (surfaceCols - 1);
      const zRatio = row / (surfaceRows - 1);
      positions.push(xRatio * width - width / 2, 0, zRatio * depth - depth / 2);
    }
  }
  for (let row = 0; row < surfaceRows - 1; row += 1) {
    for (let col = 0; col < surfaceCols - 1; col += 1) {
      const a = row * surfaceCols + col;
      const b = a + 1;
      const c = a + surfaceCols;
      const d = c + 1;
      indices.push(a, c, b, b, c, d);
    }
  }
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
  geometry.setIndex(indices);
  return geometry;
}

function addContourLines(group, values) {
  [0.85, 1.25, 1.65, 2.05, 2.45].forEach((level) => {
    const points = [];
    values.forEach((row, rowIndex) => {
      row.forEach((value, colIndex) => {
        if (Math.abs(value - level) < 0.055) {
          points.push(
            new THREE.Vector3(
              (colIndex / (surfaceCols - 1)) * 6.4 - 3.2,
              value + 0.025,
              (rowIndex / (surfaceRows - 1)) * 4.7 - 2.35,
            ),
          );
        }
      });
    });
    if (points.length < 2) return;
    group.add(
      new THREE.Line(
        new THREE.BufferGeometry().setFromPoints(points),
        new THREE.LineBasicMaterial({ color: 0xffffff, transparent: true, opacity: 0.56 }),
      ),
    );
  });
}

function addScenarioMarkers(group, interactive, values) {
  [
    { label: "Base case", row: 7, col: 7, value: "$142k ARR", color: 0x182234 },
    { label: "Expansion-led upside", row: 3, col: 12, value: "$182k ARR", color: 0xdf604d },
    { label: "Retention pressure", row: 9, col: 4, value: "$119k ARR", color: 0x3577d4 },
  ].forEach((marker) => {
    const x = (marker.col / (surfaceCols - 1)) * 6.4 - 3.2;
    const z = (marker.row / (surfaceRows - 1)) * 4.7 - 2.35;
    const y = values[marker.row][marker.col] + 0.16;
    const mesh = new THREE.Mesh(
      new THREE.SphereGeometry(0.13, 24, 24),
      new THREE.MeshStandardMaterial({ color: marker.color, roughness: 0.38, metalness: 0.08 }),
    );
    mesh.position.set(x, y, z);
    mesh.castShadow = true;
    mesh.userData = {
      baseScale: mesh.scale.clone(),
      label: marker.label,
      value: marker.value,
      detail: "Scenario marker on the modeled response surface",
    };
    interactive.push(mesh);
    group.add(mesh);

    const stem = new THREE.Line(
      new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(x, 0.05, z), new THREE.Vector3(x, y, z)]),
      new THREE.LineBasicMaterial({ color: marker.color, transparent: true, opacity: 0.5 }),
    );
    group.add(stem);
  });
}

function addAxis(group, start, end, labelText) {
  const line = new THREE.Line(
    new THREE.BufferGeometry().setFromPoints([start, end]),
    new THREE.LineBasicMaterial({ color: 0x8794a8, transparent: true, opacity: 0.82 }),
  );
  group.add(line);
  if (!labelText) return;
  const label = makeLabel(labelText, "#657185", "800 18px system-ui", 260, 64);
  label.position.copy(end).lerp(start, 0.12);
  label.position.y += 0.15;
  label.scale.set(0.42, 0.14, 1);
  group.add(label);
}

function makeLegend(series) {
  const legend = new THREE.Group();
  series.forEach((item, index) => {
    const swatch = new THREE.Mesh(
      new THREE.BoxGeometry(0.16, 0.16, 0.16),
      new THREE.MeshStandardMaterial({ color: item.color, roughness: 0.5 }),
    );
    swatch.position.set(index * 1.14, 0, 0);
    legend.add(swatch);
    const label = makeLabel(item.name, "#182234", "800 18px system-ui", 160, 52);
    label.position.set(index * 1.14 + 0.36, 0, 0);
    label.scale.set(0.34, 0.12, 1);
    legend.add(label);
  });
  legend.position.x = -1.8;
  return legend;
}

function makeLabel(text, color, font, width = 256, height = 96) {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d");
  context.clearRect(0, 0, canvas.width, canvas.height);
  context.font = font;
  context.fillStyle = color;
  context.textAlign = "center";
  context.textBaseline = "middle";
  context.fillText(text, canvas.width / 2, canvas.height / 2);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, transparent: true }));
  sprite.userData.texture = texture;
  return sprite;
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function smoothstep(value) {
  const t = clamp(value, 0, 1);
  return t * t * (3 - 2 * t);
}

function mountThreeCharts() {
  document.querySelectorAll("[data-three-chart]").forEach((container) => {
    if (container.dataset.threeMounted === "true") return;
    container.dataset.threeMounted = "true";
    new ThreeChart(container, container.dataset.threeChart);
  });
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", mountThreeCharts);
} else {
  mountThreeCharts();
}

new MutationObserver(mountThreeCharts).observe(document.documentElement, {
  childList: true,
  subtree: true,
});
