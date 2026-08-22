import occtImportJs from "occt-import-js";
import occtWasmUrl from "occt-import-js/dist/occt-import-js.wasm?url";

let parserPromise;

self.addEventListener("message", async (event) => {
  const { id, buffer } = event.data;

  try {
    self.postMessage({ id, type: "progress", message: "Tessellating STEP geometry…" });
    const occt = await getParser();
    const result = occt.ReadStepFile(new Uint8Array(buffer), {
      linearUnit: "millimeter",
      linearDeflectionType: "bounding_box_ratio",
      linearDeflection: 0.001,
      angularDeflection: 0.5,
    });

    if (!result?.success || !Array.isArray(result.meshes) || result.meshes.length === 0) {
      throw new Error("OpenCascade could not find renderable geometry in this STEP file.");
    }

    self.postMessage({
      id,
      type: "result",
      meshes: result.meshes.map(serializeMesh),
    });
  } catch (error) {
    self.postMessage({
      id,
      type: "error",
      message: error instanceof Error ? error.message : String(error),
    });
  }
});

function getParser() {
  if (!parserPromise) {
    parserPromise = occtImportJs({
      locateFile(path) {
        return path.endsWith(".wasm") ? occtWasmUrl : path;
      },
    });
  }
  return parserPromise;
}

function serializeMesh(mesh) {
  return {
    name: mesh.name || "STEP part",
    color: Array.isArray(mesh.color) ? mesh.color : null,
    positions: flattenNumbers(mesh.attributes?.position?.array),
    normals: flattenNumbers(mesh.attributes?.normal?.array),
    indices: flattenNumbers(mesh.index?.array),
  };
}

function flattenNumbers(value) {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.flat ? value.flat(Infinity) : value;
}
