import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import "./styles.css";

const app = document.querySelector("#app");
const canvas = document.querySelector("#model-canvas");
const fileInput = document.querySelector("#file-input");
const fitViewButton = document.querySelector("#fit-view");
const modelName = document.querySelector("#model-name");
const statusTitle = document.querySelector("#status-title");
const statusMessage = document.querySelector("#status-message");
const statusSpinner = document.querySelector("#status-spinner");
const partsCount = document.querySelector("#parts-count");
const trianglesCount = document.querySelector("#triangles-count");
const verticesCount = document.querySelector("#vertices-count");
const sourceSize = document.querySelector("#source-size");

const scene = new THREE.Scene();
scene.background = new THREE.Color(0x111418);

const camera = new THREE.PerspectiveCamera(36, 1, 0.01, 100000);
camera.up.set(0, 0, 1);

const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
renderer.outputColorSpace = THREE.SRGBColorSpace;
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

const controls = new OrbitControls(camera, canvas);
controls.enableDamping = true;
controls.dampingFactor = 0.08;
controls.screenSpacePanning = true;

scene.add(new THREE.HemisphereLight(0xf2f6ff, 0x27303a, 2.2));
const keyLight = new THREE.DirectionalLight(0xffffff, 3.2);
keyLight.position.set(5, -7, 9);
scene.add(keyLight);
const rimLight = new THREE.DirectionalLight(0x8fb5ff, 1.4);
rimLight.position.set(-7, 4, 5);
scene.add(rimLight);

let modelGroup;
let groundGrid;
let currentBounds;
let requestId = 0;

const worker = new Worker(new URL("./step-worker.js", import.meta.url), {
  type: "module",
});

worker.addEventListener("message", (event) => {
  const message = event.data;
  if (message.id !== requestId) {
    return;
  }
  if (message.type === "progress") {
    setLoading(message.message);
    return;
  }
  if (message.type === "error") {
    setError(message.message);
    return;
  }
  if (message.type === "result") {
    displayMeshes(message.meshes);
  }
});

worker.addEventListener("error", (event) => {
  setError(event.message || "The STEP parser stopped unexpectedly.");
});

fileInput.addEventListener("change", async () => {
  const [file] = fileInput.files;
  if (!file) {
    return;
  }
  const extension = file.name.split(".").pop()?.toLowerCase();
  if (extension !== "step" && extension !== "stp") {
    setError("Choose a .step or .stp file.");
    return;
  }
  await loadStep(file.name, await file.arrayBuffer(), file.size);
});

fitViewButton.addEventListener("click", () => {
  if (currentBounds) {
    fitCamera(currentBounds);
  }
});

const resizeObserver = new ResizeObserver(() => resizeRenderer());
resizeObserver.observe(canvas.parentElement);

renderer.setAnimationLoop(() => {
  controls.update();
  renderer.render(scene, camera);
});

loadBundledFixture();

async function loadBundledFixture() {
  try {
    const response = await fetch("/counterbore.step");
    if (!response.ok) {
      throw new Error(`Fixture request failed with status ${response.status}.`);
    }
    const buffer = await response.arrayBuffer();
    await loadStep("counterbore.step", buffer, buffer.byteLength);
  } catch (error) {
    setError(error instanceof Error ? error.message : String(error));
  }
}

async function loadStep(name, buffer, bytes) {
  requestId += 1;
  modelName.textContent = name;
  sourceSize.textContent = formatBytes(bytes);
  resetMetrics();
  setLoading("Reading STEP entities…");
  worker.postMessage({ id: requestId, buffer }, [buffer]);
}

function displayMeshes(meshes) {
  clearModel();
  modelGroup = new THREE.Group();
  modelGroup.name = "STEP model";

  let vertexTotal = 0;
  let triangleTotal = 0;

  for (const [index, meshData] of meshes.entries()) {
    if (meshData.positions.length < 9 || meshData.indices.length < 3) {
      continue;
    }

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute(
      "position",
      new THREE.Float32BufferAttribute(meshData.positions, 3),
    );
    if (meshData.normals.length === meshData.positions.length) {
      geometry.setAttribute(
        "normal",
        new THREE.Float32BufferAttribute(meshData.normals, 3),
      );
    } else {
      geometry.computeVertexNormals();
    }
    geometry.setIndex(meshData.indices);
    geometry.computeBoundingBox();
    geometry.computeBoundingSphere();

    const material = new THREE.MeshStandardMaterial({
      color: normalizeColor(meshData.color, index),
      metalness: 0.08,
      roughness: 0.72,
      side: THREE.DoubleSide,
    });
    const part = new THREE.Mesh(geometry, material);
    part.name = meshData.name;
    modelGroup.add(part);

    const edges = new THREE.LineSegments(
      new THREE.EdgesGeometry(geometry, 28),
      new THREE.LineBasicMaterial({ color: 0x171b20, transparent: true, opacity: 0.58 }),
    );
    part.add(edges);

    vertexTotal += meshData.positions.length / 3;
    triangleTotal += meshData.indices.length / 3;
  }

  if (modelGroup.children.length === 0) {
    setError("The STEP parser returned no drawable triangles.");
    return;
  }

  scene.add(modelGroup);
  currentBounds = new THREE.Box3().setFromObject(modelGroup);
  addGroundGrid(currentBounds);
  fitCamera(currentBounds);

  partsCount.textContent = formatNumber(modelGroup.children.length);
  trianglesCount.textContent = formatNumber(triangleTotal);
  verticesCount.textContent = formatNumber(vertexTotal);
  fitViewButton.disabled = false;
  setReady(
    `${formatNumber(triangleTotal)} triangles rendered locally from ${formatNumber(modelGroup.children.length)} part${modelGroup.children.length === 1 ? "" : "s"}.`,
  );
}

function clearModel() {
  currentBounds = null;
  fitViewButton.disabled = true;
  if (modelGroup) {
    scene.remove(modelGroup);
    modelGroup.traverse((object) => {
      object.geometry?.dispose();
      if (Array.isArray(object.material)) {
        object.material.forEach((material) => material.dispose());
      } else {
        object.material?.dispose();
      }
    });
    modelGroup = undefined;
  }
  if (groundGrid) {
    scene.remove(groundGrid);
    groundGrid.geometry.dispose();
    groundGrid.material.dispose();
    groundGrid = undefined;
  }
}

function addGroundGrid(bounds) {
  const size = new THREE.Vector3();
  bounds.getSize(size);
  const center = new THREE.Vector3();
  bounds.getCenter(center);
  const span = Math.max(size.x, size.y, size.z, 1);
  const gridSize = span * 3;
  groundGrid = new THREE.GridHelper(gridSize, 20, 0x39414a, 0x242a30);
  groundGrid.rotation.x = Math.PI / 2;
  groundGrid.position.set(center.x, center.y, bounds.min.z - span * 0.015);
  groundGrid.material.transparent = true;
  groundGrid.material.opacity = 0.5;
  scene.add(groundGrid);
}

function fitCamera(bounds) {
  const size = new THREE.Vector3();
  const center = new THREE.Vector3();
  bounds.getSize(size);
  bounds.getCenter(center);
  const maxDimension = Math.max(size.x, size.y, size.z, 1);
  const halfFov = THREE.MathUtils.degToRad(camera.fov * 0.5);
  const distance = (maxDimension * 0.68) / Math.tan(halfFov);
  const direction = new THREE.Vector3(1.15, -1.35, 0.9).normalize();

  camera.position.copy(center).addScaledVector(direction, distance);
  camera.near = Math.max(distance / 1000, 0.001);
  camera.far = Math.max(distance * 100, 1000);
  camera.updateProjectionMatrix();
  controls.target.copy(center);
  controls.update();
}

function resizeRenderer() {
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  if (width === 0 || height === 0) {
    return;
  }
  renderer.setSize(width, height, false);
  camera.aspect = width / height;
  camera.updateProjectionMatrix();
}

function setLoading(message) {
  app.dataset.state = "loading";
  statusTitle.textContent = "Loading model";
  statusMessage.textContent = message;
  statusSpinner.hidden = false;
}

function setReady(message) {
  app.dataset.state = "ready";
  statusTitle.textContent = "Model ready";
  statusMessage.textContent = message;
  statusSpinner.hidden = true;
}

function setError(message) {
  app.dataset.state = "error";
  statusTitle.textContent = "Could not open STEP file";
  statusMessage.textContent = message;
  statusSpinner.hidden = true;
  clearModel();
  resetMetrics();
}

function resetMetrics() {
  partsCount.textContent = "—";
  trianglesCount.textContent = "—";
  verticesCount.textContent = "—";
}

function normalizeColor(color, index) {
  if (!Array.isArray(color) || color.length < 3) {
    const fallback = [0x9bb7c9, 0xa8b8a2, 0xc0aa92, 0xa7a1c6];
    return fallback[index % fallback.length];
  }
  const scale = Math.max(...color.slice(0, 3)) > 1 ? 1 / 255 : 1;
  return new THREE.Color(
    color[0] * scale,
    color[1] * scale,
    color[2] * scale,
  );
}

function formatBytes(bytes) {
  if (!Number.isFinite(bytes)) {
    return "—";
  }
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatNumber(value) {
  return new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 }).format(value);
}
