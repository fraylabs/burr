/*
MIT License

Copyright (c) 2026 look contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
// Camera, controls and lighting adapted from Look src/ui.rs (MIT, Stefan Golas).
use look::{
    config::{parse_hex_color, LightingConfig},
    scene::CompiledScene,
};
use std::{
    collections::{BTreeSet, HashMap},
    result::Result,
};
/// Generates the local viewer page; definition buffers travel separately.
///
/// Lighting follows the CLI `LightingConfig` (ambient, direction, intensity,
/// colour) and `background` so a `--gui` view matches the `render` settings.
pub fn generate_html_viewer(
    scene: &CompiledScene,
    title: &str,
    lighting: &LightingConfig,
    background: &str,
    assets: &crate::cache::ViewerCache,
    focus: Option<&[usize]>,
    motion: Option<&crate::motion::PreparedMotion>,
) -> Result<String, String> {
    for (index, instance) in scene.instances.iter().enumerate() {
        if instance.geometry >= scene.geometries.len() {
            return Err(format!(
                "Component occurrence {index} references missing geometry {}.",
                instance.geometry
            ));
        }
    }
    let used = scene
        .instances
        .iter()
        .map(|instance| instance.geometry)
        .collect::<BTreeSet<_>>();
    let mut definitions = Vec::with_capacity(used.len());
    let mut definition_index = HashMap::with_capacity(used.len());
    for geometry in used {
        definition_index.insert(geometry, definitions.len());
        definitions.push(assets.store_mesh(&scene.geometries[geometry])?);
    }
    assets.maintain_mesh_cache();
    let occurrences = scene.instances.iter().enumerate().map(|(index, instance)| {
        let color = match focus {
            Some(indexes) if indexes.first() == Some(&index) => [1.0, 0.34, 0.08, 1.0],
            Some(indexes) if indexes.contains(&index) => [0.12, 0.76, 0.94, 1.0],
            Some(_) => [0.28, 0.31, 0.33, 0.0],
            None => scene.materials.get(instance.material)
                .map(|m| m.base_color_factor).unwrap_or([1.0; 4]),
        };
        serde_json::json!({ "geometry": definition_index[&instance.geometry], "transform": instance.transform.to_cols_array(),
            "normal": instance.normal_transform.to_cols_array(), "color": color, "id": index })
    }).collect::<Vec<_>>();
    let manifest = serde_json::json!({ "definitions": definitions, "occurrences": occurrences, "highlight": focus.is_some() });
    let manifest = manifest.to_string();
    let num_triangles: usize = scene
        .instances
        .iter()
        .map(|i| scene.geometries[i.geometry].indices.len() / 3)
        .sum();
    let num_vertices: usize = scene
        .instances
        .iter()
        .map(|i| scene.geometries[i.geometry].vertices.len())
        .sum();
    let title = title
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    let bounds = motion.map_or(scene.bounds, |m| m.bounds);
    let center = [
        (bounds.min[0] + bounds.max[0]) * 0.5,
        (bounds.min[1] + bounds.max[1]) * 0.5,
        (bounds.min[2] + bounds.max[2]) * 0.5,
    ];
    let fit_radius = motion
        .map_or(scene.fit_radius, |m| m.fit_radius)
        .max(1.0e-3);

    let direction = glam::Vec3::from_array(lighting.direction).normalize_or_zero();
    let fill_direction =
        glam::Vec3::new(-direction.x * 0.4, 0.5, -direction.z * 0.4).normalize_or_zero();
    let light_color = parse_hex_color(&lighting.color).map_err(|e| e.to_string())?;
    let bg = parse_hex_color(background).map_err(|e| e.to_string())?;

    let fs_source = format!(
        r#"#version 300 es
            precision highp float;
            in vec3 vNormal;
            in vec3 vFragPos;
            in vec4 vColor;
            uniform bool uHighlight;
            uniform vec3 uCameraPos;
            out vec4 fragColor;
            void main() {{
                vec3 N = normalize(vNormal);
                vec3 lightDir = normalize(vec3({lx}, {ly}, {lz}));
                vec3 lightColor = vec3({lr}, {lg}, {lb});
                float intensity = {intensity};
                float ambient = {ambient};

                vec3 fillDir = normalize(vec3({fx}, {fy}, {fz}));
                float diff = max(dot(N, lightDir), 0.0);
                float fill = max(dot(N, fillDir), 0.0) * 0.25;

                vec3 col = vColor.rgb * (lightColor * (ambient + (diff + fill) * intensity));

                vec3 viewDir = normalize(uCameraPos - vFragPos);
                vec3 halfDir = normalize(lightDir + viewDir);
                float spec = pow(max(dot(N, halfDir), 0.0), 32.0) * 0.25 * intensity;
                col += vec3(spec) * lightColor;

                fragColor = vec4(col, 1.0);
                // X-ray context stays translucent; selected occurrences are vivid and opaque.
                if (uHighlight && fragColor.a < 0.99) {{
                    fragColor = vec4(vColor.rgb, vColor.a > 0.5 ? 1.0 : 0.06);
                }}
            }}
        "#,
        lx = direction.x,
        ly = direction.y,
        lz = direction.z,
        // Computed here rather than negated inside the shader source: a
        // negative component would emit `--0.26` and fail to compile.
        fx = fill_direction.x,
        fy = fill_direction.y,
        fz = fill_direction.z,
        lr = light_color[0],
        lg = light_color[1],
        lb = light_color[2],
        intensity = lighting.intensity,
        ambient = lighting.ambient,
    );

    let format_badge = if title.to_lowercase().ends_with(".step")
        || title.to_lowercase().ends_with(".stp")
    {
        "STEP B-REP"
    } else if title.to_lowercase().ends_with(".glb") || title.to_lowercase().ends_with(".gltf") {
        "glTF / GLB"
    } else if title.to_lowercase().ends_with(".stl") {
        "STL MESH"
    } else {
        "3D MODEL"
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} - Burr 3D Interactive Viewer</title>
    <script>
        function burrReportViewerError(message) {{
            window.parent.postMessage({{type: 'burr:viewer-error', message,
                loadId: new URLSearchParams(location.search).get('load')}}, location.origin);
        }}
        window.addEventListener('error', event => burrReportViewerError(event.message));
    </script>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; user-select: none; }}
        body {{
            background-color: #0c0d12;
            color: #e2e8f0;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            overflow: hidden;
            width: 100vw;
            height: 100vh;
        }}
        #canvas-container {{
            width: 100%;
            height: 100%;
            position: absolute;
            top: 0;
            left: 0;
        }}
        canvas {{ width: 100%; height: 100%; display: block; }}
        
        .header-bar {{
            position: absolute;
            top: 16px;
            left: 16px;
            background: rgba(18, 20, 29, 0.75);
            backdrop-filter: blur(12px);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 12px;
            padding: 14px 20px;
            display: flex;
            align-items: center;
            gap: 16px;
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
            pointer-events: auto;
            z-index: 10;
        }}
        .model-title {{
            font-weight: 700;
            font-size: 16px;
            color: #f8fafc;
            letter-spacing: -0.01em;
        }}
        .format-badge {{
            background: linear-gradient(135deg, #3b82f6, #2563eb);
            color: #ffffff;
            font-size: 11px;
            font-weight: 700;
            padding: 4px 8px;
            border-radius: 6px;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}
        .stat-item {{
            font-size: 12px;
            color: #94a3b8;
            display: flex;
            gap: 4px;
        }}
        .stat-value {{
            color: #cbd5e1;
            font-weight: 600;
        }}

        .toolbar {{
            position: absolute;
            bottom: 24px;
            left: 50%;
            transform: translateX(-50%);
            background: rgba(18, 20, 29, 0.85);
            backdrop-filter: blur(12px);
            border: 1px solid rgba(255, 255, 255, 0.12);
            border-radius: 30px;
            padding: 6px 12px;
            display: flex;
            gap: 6px;
            box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
            z-index: 10;
        }}
        .btn {{
            background: transparent;
            border: none;
            color: #94a3b8;
            font-weight: 600;
            font-size: 13px;
            padding: 8px 14px;
            border-radius: 20px;
            cursor: pointer;
            transition: all 0.15s ease;
        }}
        .btn:hover {{
            background: rgba(255, 255, 255, 0.1);
            color: #f8fafc;
        }}
        .btn:active {{
            transform: scale(0.96);
        }}
        
        .legend {{
            position: absolute;
            bottom: 24px;
            right: 24px;
            background: rgba(18, 20, 29, 0.65);
            backdrop-filter: blur(8px);
            border: 1px solid rgba(255, 255, 255, 0.08);
            border-radius: 8px;
            padding: 10px 14px;
            font-size: 11px;
            color: #64748b;
            line-height: 1.6;
            z-index: 10;
        }}
        .legend kbd {{
            background: rgba(255, 255, 255, 0.1);
            color: #cbd5e1;
            padding: 2px 5px;
            border-radius: 4px;
            font-family: inherit;
        }}
    </style>
</head>
<body>
    <div id="canvas-container">
        <canvas id="gl-canvas"></canvas>
    </div>

    <div class="header-bar">
        <div>
            <div class="model-title">{title}</div>
            <div style="margin-top: 4px; display: flex; gap: 12px; align-items: center;">
                <span class="format-badge">{format_badge}</span>
                <span class="stat-item">Triangles: <span class="stat-value">{num_triangles}</span></span>
                <span class="stat-item">Vertices: <span class="stat-value">{num_vertices}</span></span>
                <span class="stat-item">Geometries: <span class="stat-value">{unique_geometries}</span></span>
            </div>
        </div>
    </div>

    <div class="toolbar">
        <button class="btn" onclick="setPresetView('iso')">Iso</button>
        <button class="btn" onclick="setPresetView('front')">Front</button>
        <button class="btn" onclick="setPresetView('top')">Top</button>
        <button class="btn" onclick="setPresetView('right')">Right</button>
        <button class="btn" onclick="resetView()">Reset</button>
    </div>

    <div class="legend">
        <div><kbd>Left Drag</kbd> Rotate model</div>
        <div><kbd>Scroll Wheel</kbd> Zoom in / out</div>
        <div><kbd>Right / Shift Drag</kbd> Pan view</div>
    </div>

    <script>
        const burrManifest = {manifest};
        // Keep CPU mesh buffers only until their definition has been uploaded.
        let burrMeshes = [];
        const modelCenter = [{center_x}, {center_y}, {center_z}];
        const fitRadius = {fit_radius};

        let yaw = Math.PI / 4;
        let pitch = Math.PI / 6;
        let distance = fitRadius * 2.5;
        let target = [...modelCenter];

        const canvas = document.getElementById('gl-canvas');
        const gl = canvas.getContext('webgl2', {{ antialias: true }});
        if (!gl) throw new Error('WebGL 2.0 is required');
        canvas.addEventListener('webglcontextlost', () => burrReportViewerError('The browser lost the model display.'));

        const vsSource = `#version 300 es
            in vec3 aPosition;
            in vec3 aNormal;
            in vec4 aColor;
            in mat4 aTransform;
            in mat3 aNormalTransform;
            in vec4 aFactor;
            in float aBurrInstance;
            uniform bool uHighlight;
            uniform mat4 uMVP;
            uniform mat4 uModel;
            out vec3 vNormal;
            out vec3 vFragPos;
            out vec4 vColor;
            void main() {{
                vNormal = normalize(aNormalTransform * aNormal);
                vFragPos = vec3(aTransform * vec4(aPosition, 1.0));
                vColor = uHighlight ? aFactor : aColor * aFactor;
                gl_Position = uMVP * aTransform * vec4(aPosition, 1.0);
            }}
        `;

        const fsSource = `{fs_source}`;

        function createShader(gl, type, source) {{
            const shader = gl.createShader(type);
            gl.shaderSource(shader, source);
            gl.compileShader(shader);
            if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {{
                const kind = type === gl.VERTEX_SHADER ? 'vertex' : 'fragment';
                throw new Error(kind + ' shader failed to compile: ' + gl.getShaderInfoLog(shader));
            }}
            return shader;
        }}

        const program = gl.createProgram();
        gl.attachShader(program, createShader(gl, gl.VERTEX_SHADER, vsSource));
        gl.attachShader(program, createShader(gl, gl.FRAGMENT_SHADER, fsSource));
        gl.linkProgram(program);
        if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {{
            throw new Error('shader program failed to link: ' + gl.getProgramInfoLog(program));
        }}

        const vao = null; // Each definition owns one VAO and one GPU mesh.
        async function burrLoadMeshes() {{
            const byGeometry = burrManifest.definitions.map(() => []);
            for (const occurrence of burrManifest.occurrences)
                byGeometry[occurrence.geometry].push(occurrence);
            // Upload one payload at a time to bound transient browser mesh memory.
            for (let geometry = 0; geometry < burrManifest.definitions.length; geometry++) {{
                const definition = burrManifest.definitions[geometry];
                const occurrences = byGeometry[geometry];
                if (!occurrences.length) continue;
                const response = await fetch('/mesh/' + definition.id);
                if (!response.ok) throw new Error('Could not load mesh: ' + response.status);
                const bytes = await response.arrayBuffer();
                if (bytes.byteLength !== definition.vertices * definition.stride + definition.indices * 4)
                    throw new Error('Mesh length did not match its manifest');
                const meshVao = gl.createVertexArray();
                gl.bindVertexArray(meshVao);
                const vertices = gl.createBuffer();
                gl.bindBuffer(gl.ARRAY_BUFFER, vertices);
                gl.bufferData(gl.ARRAY_BUFFER, new Uint8Array(bytes, 0, definition.vertices * definition.stride), gl.STATIC_DRAW);
                for (const [name, size, offset] of [['aPosition',3,0], ['aNormal',3,12], ...(definition.color ? [] : [['aColor',4,24]])]) {{
                    const location = gl.getAttribLocation(program, name);
                    gl.enableVertexAttribArray(location);
                    gl.vertexAttribPointer(location, size, gl.FLOAT, false, definition.stride, offset);
                }}
                const elements = gl.createBuffer();
                gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, elements);
                gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, new Uint8Array(bytes, definition.vertices * definition.stride), gl.STATIC_DRAW);
                const instanceData = new Float32Array(occurrences.length * 30);
                occurrences.forEach((instance, index) => {{
                    instanceData.set(instance.transform, index * 30);
                    instanceData.set(instance.normal, index * 30 + 16);
                    instanceData.set(instance.color, index * 30 + 25);
                    instanceData[index * 30 + 29] = instance.id;
                }});
                const instances = gl.createBuffer();
                gl.bindBuffer(gl.ARRAY_BUFFER, instances);
                gl.bufferData(gl.ARRAY_BUFFER, instanceData, gl.STATIC_DRAW);
                for (const [name, columns, size, offset] of [['aTransform',4,4,0], ['aNormalTransform',3,3,64], ['aFactor',1,4,100], ['aBurrInstance',1,1,116]]) {{
                    const location = gl.getAttribLocation(program, name);
                    if (location < 0) continue;
                    for (let column = 0; column < columns; column++) {{
                        gl.enableVertexAttribArray(location + column);
                        gl.vertexAttribPointer(location + column, size, gl.FLOAT, false, 120, offset + column * size * 4);
                        gl.vertexAttribDivisor(location + column, 1);
                    }}
                }}
                burrMeshes.push({{ vao: meshVao, count: definition.indices, instances: occurrences.length, color: definition.color }});
            }}
            if (gl.getError() !== gl.NO_ERROR) throw new Error('The browser could not prepare the model.');
        }}

        const uMVPLoc = gl.getUniformLocation(program, 'uMVP');
        const uModelLoc = gl.getUniformLocation(program, 'uModel');
        const uCamPosLoc = gl.getUniformLocation(program, 'uCameraPos');
        gl.useProgram(program);
        gl.uniform1i(gl.getUniformLocation(program, 'uHighlight'), burrManifest.highlight);

        function mat4Identity() {{
            return new Float32Array([1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]);
        }}
        function mat4Perspective(fovy, aspect, near, far) {{
            const f = 1.0 / Math.tan(fovy / 2);
            const nf = 1.0 / (near - far);
            return new Float32Array([
                f / aspect, 0, 0, 0,
                0, f, 0, 0,
                0, 0, (far + near) * nf, -1,
                0, 0, (2 * far * near) * nf, 0
            ]);
        }}
        function mat4LookAt(eye, target, up) {{
            const z0 = eye[0] - target[0], z1 = eye[1] - target[1], z2 = eye[2] - target[2];
            let len = 1 / Math.hypot(z0, z1, z2);
            const zx = z0 * len, zy = z1 * len, zz = z2 * len;

            const xx = up[1] * zz - up[2] * zy, xy = up[2] * zx - up[0] * zz, xz = up[0] * zy - up[1] * zx;
            len = 1 / Math.hypot(xx, xy, xz);
            const rx = xx * len, ry = xy * len, rz = xz * len;

            const yx = zy * rz - zz * ry, yy = zz * rx - zx * rz, yz = zx * ry - zy * rx;

            return new Float32Array([
                rx, yx, zx, 0,
                ry, yy, zy, 0,
                rz, yz, zz, 0,
                -(rx * eye[0] + ry * eye[1] + rz * eye[2]),
                -(yx * eye[0] + yy * eye[1] + yz * eye[2]),
                -(zx * eye[0] + zy * eye[1] + zz * eye[2]), 1
            ]);
        }}
        function mat4Multiply(a, b) {{
            const out = new Float32Array(16);
            for (let i = 0; i < 4; i++) {{
                for (let j = 0; j < 4; j++) {{
                    out[j * 4 + i] =
                        a[i] * b[j * 4] +
                        a[i + 4] * b[j * 4 + 1] +
                        a[i + 8] * b[j * 4 + 2] +
                        a[i + 12] * b[j * 4 + 3];
                }}
            }}
            return out;
        }}

        function render() {{
            const width = canvas.clientWidth;
            const height = canvas.clientHeight;
            if (canvas.width !== width || canvas.height !== height) {{
                canvas.width = width;
                canvas.height = height;
                gl.viewport(0, 0, width, height);
            }}

            gl.enable(gl.DEPTH_TEST);
            gl.clearColor({bg_r}, {bg_g}, {bg_b}, 1.0);
            gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

            const camX = target[0] + distance * Math.cos(pitch) * Math.sin(yaw);
            const camY = target[1] + distance * Math.sin(pitch);
            const camZ = target[2] + distance * Math.cos(pitch) * Math.cos(yaw);
            const camPos = [camX, camY, camZ];

            const proj = mat4Perspective(Math.PI / 4, width / height, fitRadius * 0.01, fitRadius * 20.0);
            const view = mat4LookAt(camPos, target, [0, 1, 0]);
            const model = mat4Identity();

            const mvp = mat4Multiply(proj, mat4Multiply(view, model));

            gl.useProgram(program);
            gl.uniformMatrix4fv(uMVPLoc, false, mvp);
            gl.uniformMatrix4fv(uModelLoc, false, model);
            gl.uniform3fv(uCamPosLoc, camPos);

            gl.bindVertexArray(vao);
            gl.drawElements(gl.TRIANGLES, indices.length, gl.UNSIGNED_INT, 0);

            requestAnimationFrame(render);
        }}

        let isDragging = false;
        let isPanning = false;
        let lastMouseX = 0;
        let lastMouseY = 0;

        canvas.addEventListener('mousedown', (e) => {{
            isDragging = true;
            isPanning = e.button === 2 || e.shiftKey;
            lastMouseX = e.clientX;
            lastMouseY = e.clientY;
        }});

        window.addEventListener('mouseup', () => {{ isDragging = false; }});
        canvas.addEventListener('contextmenu', (e) => e.preventDefault());

        canvas.addEventListener('mousemove', (e) => {{
            if (!isDragging) return;
            const dx = e.clientX - lastMouseX;
            const dy = e.clientY - lastMouseY;
            lastMouseX = e.clientX;
            lastMouseY = e.clientY;

            if (isPanning) {{
                const panSpeed = distance * 0.0015;
                target[0] -= dx * panSpeed * Math.cos(yaw);
                target[1] += dy * panSpeed;
                target[2] += dx * panSpeed * Math.sin(yaw);
            }} else {{
                yaw -= dx * 0.006;
                pitch += dy * 0.006;
                const limit = Math.PI / 2 - 0.05;
                pitch = Math.max(-limit, Math.min(limit, pitch));
            }}
        }});

        canvas.addEventListener('wheel', (e) => {{
            e.preventDefault();
            const zoomFactor = e.deltaY > 0 ? 1.1 : 0.9;
            distance = Math.max(fitRadius * 0.1, Math.min(fitRadius * 15.0, distance * zoomFactor));
        }}, {{ passive: false }});

        let touchStartDist = 0;
        canvas.addEventListener('touchstart', (e) => {{
            if (e.touches.length === 1) {{
                isDragging = true;
                lastMouseX = e.touches[0].clientX;
                lastMouseY = e.touches[0].clientY;
            }} else if (e.touches.length === 2) {{
                touchStartDist = Math.hypot(
                    e.touches[0].clientX - e.touches[1].clientX,
                    e.touches[0].clientY - e.touches[1].clientY
                );
            }}
        }});
        canvas.addEventListener('touchmove', (e) => {{
            if (e.touches.length === 1 && isDragging) {{
                const dx = e.touches[0].clientX - lastMouseX;
                const dy = e.touches[0].clientY - lastMouseY;
                lastMouseX = e.touches[0].clientX;
                lastMouseY = e.touches[0].clientY;
                yaw -= dx * 0.006;
                pitch += dy * 0.006;
            }} else if (e.touches.length === 2) {{
                const dist = Math.hypot(
                    e.touches[0].clientX - e.touches[1].clientX,
                    e.touches[0].clientY - e.touches[1].clientY
                );
                if (touchStartDist > 0) {{
                    const factor = touchStartDist / dist;
                    distance *= factor;
                    touchStartDist = dist;
                }}
            }}
        }});
        canvas.addEventListener('touchend', () => {{ isDragging = false; }});

        function setPresetView(name) {{
            target = [...modelCenter];
            distance = fitRadius * 2.5;
            if (name === 'iso') {{ yaw = Math.PI / 4; pitch = Math.PI / 6; }}
            else if (name === 'front') {{ yaw = 0; pitch = 0; }}
            else if (name === 'top') {{ yaw = 0; pitch = Math.PI / 2 - 0.01; }}
            else if (name === 'right') {{ yaw = Math.PI / 2; pitch = 0; }}
        }}

        function resetView() {{
            setPresetView('iso');
        }}

        burrLoadMeshes().then(() => {{
            requestAnimationFrame(render);
        }}).catch(error => {{
            document.body.textContent = 'Could not open model: ' + error.message;
            burrReportViewerError(error.message);
        }});
    </script>
</body>
</html>"#,
        title = title,
        manifest = manifest,
        format_badge = format_badge,
        num_triangles = num_triangles,
        num_vertices = num_vertices,
        unique_geometries = scene.statistics.unique_geometries,
        center_x = center[0],
        center_y = center[1],
        center_z = center[2],
        fit_radius = fit_radius,
        fs_source = fs_source,
        bg_r = bg[0],
        bg_g = bg[1],
        bg_b = bg[2],
    );

    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;
    use look::{config::UpAxis, scene::compile_scene, timing::Timings};
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn occurrences_and_highlights_reuse_definition_payloads() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/viewer/models/enclosure/counterbore.step");
        let mut scene = compile_scene(&path, UpAxis::Z, &mut Timings::default()).unwrap();
        let original = scene.instances[0].clone();
        scene.instances.push(original);
        scene.instances[1].transform *= glam::Mat4::from_translation(glam::Vec3::X * 10.0);
        let temp = tempdir().unwrap();
        let cache = crate::cache::ViewerCache::at(temp.path().join("cache"));
        let html = generate_html_viewer(
            &scene,
            "part.step",
            &LightingConfig::default(),
            "#0c0d10",
            &cache,
            None,
            None,
        )
        .unwrap();
        let parse = |html: &str| {
            let json = html
                .split_once("const burrManifest = ")
                .unwrap()
                .1
                .split_once(";\n")
                .unwrap()
                .0;
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        };
        let manifest = parse(&html);
        assert_eq!(manifest["occurrences"].as_array().unwrap().len(), 2);
        assert_ne!(
            manifest["occurrences"][0]["transform"],
            manifest["occurrences"][1]["transform"]
        );
        let highlighted = generate_html_viewer(
            &scene,
            "part.step",
            &LightingConfig::default(),
            "#0c0d10",
            &cache,
            Some(&[0, 1]),
            None,
        )
        .unwrap();
        let highlighted_manifest = parse(&highlighted);
        assert_eq!(manifest["definitions"], highlighted_manifest["definitions"]);
        assert_ne!(
            highlighted_manifest["occurrences"][0]["color"],
            highlighted_manifest["occurrences"][1]["color"]
        );
        assert!(html.len() < 100_000);
        let id = manifest["definitions"][0]["id"].as_str().unwrap();
        let mesh = std::fs::OpenOptions::new()
            .write(true)
            .open(cache.mesh_path(id).unwrap())
            .unwrap();
        mesh.set_times(std::fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH))
            .unwrap();
        assert!(cache.meshes_available(&html));
        assert!(
            mesh.metadata()
                .unwrap()
                .modified()
                .unwrap()
                .elapsed()
                .unwrap()
                .as_secs()
                < 30
        );
        let id = manifest["definitions"][0]["id"].as_str().unwrap();
        std::fs::remove_file(cache.mesh_path(id).unwrap()).unwrap();
        assert!(!cache.meshes_available(&html));
        let rebuilt = generate_html_viewer(
            &scene,
            "part.step",
            &LightingConfig::default(),
            "#0c0d10",
            &cache,
            None,
            None,
        )
        .unwrap();
        assert_eq!(html, rebuilt);
        assert!(cache.meshes_available(&rebuilt));
    }

    #[test]
    fn unavailable_persistent_cache_still_serves_local_meshes() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/viewer/models/enclosure/counterbore.step");
        let scene = compile_scene(&path, UpAxis::Z, &mut Timings::default()).unwrap();
        let temp = tempdir().unwrap();
        let blocked = temp.path().join("not-a-directory");
        std::fs::write(&blocked, "cache unavailable").unwrap();
        let cache = crate::cache::ViewerCache::at(blocked);
        let definition = cache.store_mesh(&scene.geometries[0]).unwrap();
        let file = cache.mesh_path(definition["id"].as_str().unwrap()).unwrap();
        assert!(file.is_file());
        assert!(!file.starts_with(temp.path()));
        std::fs::remove_file(file).unwrap();
    }

    #[test]
    fn constant_color_payloads_do_not_repeat_rgba_per_vertex() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/viewer/models/enclosure/counterbore.step");
        let mut scene = compile_scene(&path, UpAxis::Z, &mut Timings::default()).unwrap();
        let temp = tempdir().unwrap();
        let cache = crate::cache::ViewerCache::at(temp.path().join("cache"));
        let geometry = &mut scene.geometries[0];
        geometry.source_attributes = None;
        let constant = cache.store_mesh(geometry).unwrap();
        assert_eq!(constant["stride"], 24);
        let mut attributes = vec![
            look::scene::SourceVertexAttributes {
                tex_coord_0: [0.0; 2],
                tex_coord_1: [0.0; 2],
                color: [1.0; 4],
            };
            geometry.vertices.len()
        ];
        attributes[0].color = [0.25, 0.5, 0.75, 1.0];
        geometry.source_attributes = Some(attributes);
        let varied = cache.store_mesh(geometry).unwrap();
        assert_eq!(varied["stride"], 40);
        assert!(varied["color"].is_null());
        let bytes =
            std::fs::read(cache.mesh_path(varied["id"].as_str().unwrap()).unwrap()).unwrap();
        assert_eq!(f32::from_le_bytes(bytes[24..28].try_into().unwrap()), 0.25);
        let constant_size =
            std::fs::metadata(cache.mesh_path(constant["id"].as_str().unwrap()).unwrap())
                .unwrap()
                .len();
        assert_eq!(
            bytes.len() as u64 - constant_size,
            geometry.vertices.len() as u64 * 16
        );
    }

    #[test]
    fn invalid_geometry_reference_is_an_error() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/viewer/models/enclosure/counterbore.step");
        let mut scene = compile_scene(&path, UpAxis::Z, &mut Timings::default()).unwrap();
        scene.instances[0].geometry = scene.geometries.len();
        let temp = tempdir().unwrap();
        let cache = crate::cache::ViewerCache::at(temp.path().join("cache"));
        let error = generate_html_viewer(
            &scene,
            "part.step",
            &LightingConfig::default(),
            "#0c0d10",
            &cache,
            None,
            None,
        )
        .unwrap_err();
        assert!(error.contains("references missing geometry"));
    }
}
