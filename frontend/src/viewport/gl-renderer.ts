import type { MeshBuffers, VertexUpdate } from '../engine/service';
import type { OrbitCamera } from './camera';
import { mipChain, type EnvImage } from './environment';
import { invert } from './math';
import { NO_POST, PostFx, type PostSettings } from './post';

const MESH_VS = `#version 300 es
layout(location=0) in vec3 aPos;
layout(location=1) in vec3 aNrm;
layout(location=2) in float aMask;
layout(location=3) in vec3 aCol;
layout(location=4) in vec2 aMat;
uniform mat4 uViewProj;
uniform mat4 uView;
uniform mat4 uModel;
out vec3 vNrm;
out vec3 vViewPos;
out float vMask;
out vec3 vCol;
out vec2 vMat;
void main() {
  vMask = aMask;
  vCol = aCol;
  vMat = aMat;
  // uModel is the identity except while the transform gizmo previews a move, turn or scale.
  vec4 world = uModel * vec4(aPos, 1.0);
  vNrm = mat3(uView) * (transpose(inverse(mat3(uModel))) * aNrm);
  vViewPos = (uView * world).xyz;
  gl_Position = uViewProj * world;
}`;

const MESH_FS = `#version 300 es
precision highp float;
in vec3 vNrm;
in vec3 vViewPos;
in float vMask;
in vec3 vCol;
in vec2 vMat;
uniform vec3 uBase;
uniform float uOrtho;
uniform float uPainted;
uniform float uLit;
uniform vec4 uCap;
uniform vec3 uTint;
uniform float uEnvOn;
uniform sampler2D uEnv;
uniform float uEnvRot;
uniform float uEnvMaxLod;
uniform float uExposure;
uniform float uLightScale;
uniform float uTone;
uniform mat4 uView;
const float PI_ = 3.14159265;
vec3 envAt(vec3 d, float lod) {
  d = normalize(d);
  float az = atan(d.x, d.z) + uEnvRot;
  float u = fract((az + PI_) / (2.0 * PI_));
  float w = acos(clamp(d.y, -1.0, 1.0)) / PI_;
  return textureLod(uEnv, vec2(u, w), max(lod, 0.0)).rgb;
}
vec3 tonemap(vec3 c) {
  if (uTone > 1.5) return pow(clamp(c, 0.0, 1.0), vec3(1.0 / 2.2));
  if (uTone > 0.5) {
    // ACES filmic (Narkowicz's fit).
    c = clamp((c * (2.51 * c + 0.03)) / (c * (2.43 * c + 0.59) + 0.14), 0.0, 1.0);
    return pow(c, vec3(1.0 / 2.2));
  }
  return pow(c / (c + vec3(1.0)), vec3(1.0 / 2.2));
}
uniform float uImage;
uniform sampler2D uMatcap;
out vec4 outColor;
const float PI = 3.14159265;
vec3 light(vec3 n, vec3 v, vec3 l, vec3 radiance, vec3 albedo, float rough, float metal) {
  vec3 h = normalize(l + v);
  float ndl = max(dot(n, l), 0.0);
  float ndv = max(dot(n, v), 1e-3);
  float ndh = max(dot(n, h), 0.0);
  float a = rough * rough;
  float a2 = a * a;
  float d = a2 / (PI * pow(ndh * ndh * (a2 - 1.0) + 1.0, 2.0));
  float k = (rough + 1.0) * (rough + 1.0) / 8.0;
  float g = (ndl / (ndl * (1.0 - k) + k)) * (ndv / (ndv * (1.0 - k) + k));
  vec3 f0 = mix(vec3(0.04), albedo, metal);
  vec3 f = f0 + (1.0 - f0) * pow(1.0 - max(dot(h, v), 0.0), 5.0);
  vec3 spec = d * g * f / (4.0 * ndl * ndv + 1e-4);
  vec3 kd = (1.0 - f) * (1.0 - metal);
  return (kd * albedo / PI + spec) * radiance * ndl;
}
void main() {
  vec3 n = normalize(vNrm);
  vec3 v = uOrtho > 0.5 ? vec3(0.0, 0.0, 1.0) : normalize(-vViewPos);
  if (dot(n, v) < 0.0) n = -n;
  vec3 albedo = uPainted > 0.5 ? vCol : uBase;
  vec3 col;
  if (uLit > 0.5) {
    float rough = clamp(vMat.x, 0.05, 1.0);
    float metal = clamp(vMat.y, 0.0, 1.0);
    col = light(n, v, normalize(vec3(-0.45, 0.65, 0.6)), vec3(2.6, 2.45, 2.25), albedo, rough, metal);
    col += light(n, v, normalize(vec3(0.6, 0.1, 0.5)), vec3(0.55, 0.62, 0.75), albedo, rough, metal);
    col += light(n, v, normalize(vec3(0.0, 0.35, -1.0)), vec3(0.9), albedo, rough, metal);
    col *= uLightScale;
    vec3 f0 = mix(vec3(0.04), albedo, metal);
    if (uEnvOn > 0.5) {
      // Image-based light: diffuse from a blurred mip along the normal, reflections from a mip chosen by roughness,
      // weighted by an analytic fit of the split-sum BRDF (Karis), all in world space.
      mat3 toWorld = transpose(mat3(uView));
      vec3 wn = toWorld * n;
      vec3 wr = toWorld * reflect(-v, n);
      vec3 diffuse = envAt(wn, uEnvMaxLod - 1.0) * albedo * (1.0 - metal);
      const vec4 c0 = vec4(-1.0, -0.0275, -0.572, 0.022);
      const vec4 c1 = vec4(1.0, 0.0425, 1.04, -0.04);
      vec4 r = rough * c0 + c1;
      float ndv = max(dot(n, v), 1e-3);
      float a004 = min(r.x * r.x, exp2(-9.28 * ndv)) * r.x + r.y;
      vec2 ab = vec2(-1.04, 1.04) * a004 + r.zw;
      vec3 specular = envAt(wr, rough * (uEnvMaxLod - 2.0)) * (f0 * ab.x + ab.y);
      col += diffuse + specular;
    } else {
      vec3 sky = mix(vec3(0.16, 0.15, 0.14), vec3(0.42, 0.45, 0.5), n.y * 0.5 + 0.5);
      col += sky * (albedo * (1.0 - metal) + f0 * (1.0 - rough * 0.7));
    }
    col = tonemap(col * uExposure);
  } else if (uImage > 0.5) {
    // An image matcap: the view-space normal looks up the sphere picture; paint tints it.
    vec2 uv = n.xy * 0.49 + 0.5;
    vec3 cap = texture(uMatcap, vec2(uv.x, 1.0 - uv.y)).rgb;
    col = uPainted > 0.5 ? cap * albedo * 1.25 : cap;
  } else {
    vec3 r = reflect(-v, n);
    vec2 m = r.xy / (2.0 * sqrt(r.x * r.x + r.y * r.y + (r.z + 1.0) * (r.z + 1.0))) + 0.5;
    float key = clamp(dot(n, normalize(vec3(-0.45, 0.65, 0.6))), 0.0, 1.0);
    float fill = clamp(dot(n, normalize(vec3(0.6, 0.1, 0.5))), 0.0, 1.0) * 0.25;
    float rim = pow(1.0 - clamp(dot(n, v), 0.0, 1.0), 3.0) * uCap.w;
    float spec = pow(clamp(1.0 - distance(m, vec2(0.34, 0.68)) * 2.2, 0.0, 1.0), uCap.z) * uCap.y;
    float cavity = mix(0.55, 1.0, smoothstep(-0.2, 0.9, n.y * 0.5 + 0.5));
    vec3 base = albedo * uTint;
    col = base * (0.18 + key * uCap.x + fill) * cavity + (vec3(rim) + spec * mix(vec3(1.0), uTint, 0.6));
    col = pow(col, vec3(1.0 / 1.1));
  }
  col = mix(col, col * vec3(0.32, 0.34, 0.4), clamp(vMask, 0.0, 1.0) * 0.85);
  outColor = vec4(col, 1.0);
}`;

export const SHADINGS: Record<string, { lit: boolean; cap: [number, number, number, number]; tint: [number, number, number] }> = {
  clay: { lit: false, cap: [0.75, 0.45, 6, 0.35], tint: [1, 1, 1] },
  wax: { lit: false, cap: [0.85, 0.7, 10, 0.2], tint: [1.0, 0.55, 0.5] },
  pearl: { lit: false, cap: [0.6, 0.6, 4, 0.7], tint: [1.0, 0.97, 0.95] },
  bronze: { lit: false, cap: [0.45, 0.9, 14, 0.25], tint: [0.85, 0.6, 0.38] },
  gray: { lit: false, cap: [0.8, 0.25, 4, 0.2], tint: [0.78, 0.78, 0.82] },
  pbr: { lit: true, cap: [0, 0, 1, 0], tint: [1, 1, 1] },
};

const PAINT_DEFAULT = [0.8, 0.8, 0.8, 0.5, 0.0];

const LINE_VS = `#version 300 es
layout(location=0) in vec3 aPos;
layout(location=1) in vec3 aCol;
uniform mat4 uViewProj;
out vec3 vCol;
void main() { vCol = aCol; gl_Position = uViewProj * vec4(aPos, 1.0); }`;

const LINE_FS = `#version 300 es
precision mediump float;
in vec3 vCol;
out vec4 outColor;
void main() { outColor = vec4(vCol, 1.0); }`;

interface GpuMesh {
  vao: WebGLVertexArrayObject;
  buffers: WebGLBuffer[];
  count: number;
  revision: number;
  cpu: MeshBuffers;
  paintBuf: WebGLBuffer | null;
  /** The indices actually drawn (hidden triangles left out) and the source triangle of each drawn one. */
  drawn: Uint32Array;
  drawnTris: Uint32Array;
  /** De-indexed copy for showing face groups: an indexed mesh cannot give each triangle its own colour. */
  expanded: Expanded | null;
}

interface Expanded {
  vao: WebGLVertexArrayObject;
  buffers: WebGLBuffer[];
  corners: number;
  pos: Float32Array;
  nrm: Float32Array;
  msk: Float32Array;
}

/** A distinct pastel per group: hues spaced by the golden ratio so neighbouring ids never look alike. */
export function groupColor(g: number): [number, number, number] {
  const h = (g * 0.6180339887) % 1;
  const s = 0.45;
  const l = 0.66;
  const k = (n: number): number => (n + h * 12) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number): number => l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1));
  return [f(0), f(8), f(4)];
}

function compile(gl: WebGL2RenderingContext, vs: string, fs: string): WebGLProgram {
  const program = gl.createProgram();
  for (const [type, src] of [[gl.VERTEX_SHADER, vs], [gl.FRAGMENT_SHADER, fs]] as const) {
    const shader = gl.createShader(type);
    if (!shader) throw new Error('shader allocation failed');
    gl.shaderSource(shader, src);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader) ?? 'shader compile failed');
    gl.attachShader(program, shader);
  }
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program) ?? 'link failed');
  return program;
}

export class GlRenderer {
  readonly kind = 'webgl2';
  private gl: WebGL2RenderingContext;
  private meshProgram: WebGLProgram;
  private lineProgram: WebGLProgram;
  private meshes = new Map<number, GpuMesh>();
  private grid: { vao: WebGLVertexArrayObject; count: number };
  /** A 1 by 1 texture bound when no image matcap is in use, so the sampler is never incomplete. */
  private blank: WebGLTexture;
  /** The background program, its empty VAO, and a 1 by 1 black environment bound when none is in use. */
  private bgProgram: WebGLProgram;
  private bgVao: WebGLVertexArrayObject;
  private blankEnv: WebGLTexture;
  private clear: [number, number, number] = [0.11, 0.106, 0.114];
  active: number | null = null;

  constructor(canvas: HTMLCanvasElement) {
    const gl = canvas.getContext('webgl2', { antialias: true, alpha: false, preserveDrawingBuffer: false });
    if (!gl) throw new Error('WebGL2 is not available');
    this.gl = gl;
    this.meshProgram = compile(gl, MESH_VS, MESH_FS);
    this.lineProgram = compile(gl, LINE_VS, LINE_FS);
    this.grid = this.buildGrid(20, 1);
    this.bgProgram = compile(gl, BG_VS, BG_FS);
    this.bgVao = gl.createVertexArray();
    this.blankEnv = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, this.blankEnv);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([0, 0, 0, 255]));
    gl.enable(gl.DEPTH_TEST);
    this.blank = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, this.blank);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([128, 128, 128, 255]));
  }

  setClearColor(css: string): void {
    const m = /^#?([0-9a-f]{6})$/i.exec(css.trim());
    if (!m) return;
    const v = parseInt(m[1], 16);
    this.clear = [((v >> 16) & 255) / 255, ((v >> 8) & 255) / 255, (v & 255) / 255];
  }

  private buildGrid(half: number, step: number): { vao: WebGLVertexArrayObject; count: number } {
    const gl = this.gl;
    const data: number[] = [];
    for (let i = -half; i <= half; i += step) {
      const major = i === 0;
      const cx: [number, number, number] = major ? [0.75, 0.25, 0.25] : [0.24, 0.24, 0.26];
      const cz: [number, number, number] = major ? [0.25, 0.4, 0.8] : [0.24, 0.24, 0.26];
      data.push(-half, 0, i, ...cx, half, 0, i, ...cx, i, 0, -half, ...cz, i, 0, half, ...cz);
    }
    const vao = gl.createVertexArray();
    gl.bindVertexArray(vao);
    const buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(data), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 24, 0);
    gl.enableVertexAttribArray(1);
    gl.vertexAttribPointer(1, 3, gl.FLOAT, false, 24, 12);
    gl.bindVertexArray(null);
    return { vao, count: data.length / 6 };
  }

  /** Previews a transform of one object (column-major 4 by 4) until its new geometry arrives; null clears it. */
  setPreview(id: number, matrix: Float32Array | null): void {
    if (matrix) this.previews.set(id, matrix);
    else this.previews.delete(id);
  }

  private previews = new Map<number, Float32Array>();

  upload(mesh: MeshBuffers): void {
    const gl = this.gl;
    // New geometry replaces any gizmo preview in the same frame, so the object never jumps back.
    this.previews.delete(mesh.id);
    this.remove(mesh.id);
    const vao = gl.createVertexArray();
    gl.bindVertexArray(vao);
    const pos = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, pos);
    gl.bufferData(gl.ARRAY_BUFFER, mesh.positions, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 0, 0);
    const nrm = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, nrm);
    gl.bufferData(gl.ARRAY_BUFFER, mesh.normals, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(1);
    gl.vertexAttribPointer(1, 3, gl.FLOAT, false, 0, 0);
    const idx = gl.createBuffer();
    const msk = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, msk);
    gl.bufferData(gl.ARRAY_BUFFER, mesh.mask, gl.DYNAMIC_DRAW);
    gl.enableVertexAttribArray(2);
    gl.vertexAttribPointer(2, 1, gl.FLOAT, false, 0, 0);
    const tris = mesh.indices.length / 3;
    const hidden = mesh.hidden && mesh.hidden.length === tris ? mesh.hidden : null;
    let shown = tris;
    if (hidden) for (let t = 0; t < tris; t++) if (hidden[t]) shown--;
    const drawn = hidden ? new Uint32Array(shown * 3) : new Uint32Array(mesh.indices);
    const drawnTris = new Uint32Array(shown);
    for (let t = 0, k = 0; t < tris; t++) {
      if (hidden && hidden[t]) continue;
      if (hidden) drawn.set(mesh.indices.subarray(t * 3, t * 3 + 3), k * 3);
      drawnTris[k++] = t;
    }
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, idx);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, drawn, gl.STATIC_DRAW);
    const paint = mesh.paint ? new Float32Array(mesh.paint) : null;
    const paintBuf = paint ? this.paintBuffer(paint) : null;
    gl.bindVertexArray(null);
    const groups = mesh.groups && mesh.groups.length === tris ? new Uint32Array(mesh.groups) : null;
    const cpu: MeshBuffers = { ...mesh, positions: new Float32Array(mesh.positions), normals: new Float32Array(mesh.normals), mask: new Float32Array(mesh.mask), paint, groups, hidden: null };
    const gpu: GpuMesh = { vao, buffers: paintBuf ? [pos, nrm, idx, msk, paintBuf] : [pos, nrm, idx, msk], count: drawn.length, revision: mesh.revision, cpu, paintBuf, drawn, drawnTris, expanded: null };
    this.meshes.set(mesh.id, gpu);
    if (this.showGroups && groups) gpu.expanded = this.expand(gpu);
  }

  private showGroups = false;

  /** Shows each face group in its own colour (meshes without groups draw as usual). */
  setShowGroups(on: boolean): void {
    this.showGroups = on;
    for (const m of this.meshes.values()) {
      if (on && m.cpu.groups && !m.expanded) m.expanded = this.expand(m);
      if (!on && m.expanded) this.dropExpanded(m);
    }
  }

  private expand(m: GpuMesh): Expanded {
    const gl = this.gl;
    const corners = m.drawn.length;
    const col = new Float32Array(corners * 3);
    const groups = m.cpu.groups as Uint32Array;
    for (let k = 0; k < m.drawnTris.length; k++) {
      const c = groupColor(groups[m.drawnTris[k]]);
      for (let j = 0; j < 3; j++) col.set(c, (k * 3 + j) * 3);
    }
    const ex: Expanded = { vao: gl.createVertexArray(), buffers: [], corners, pos: new Float32Array(corners * 3), nrm: new Float32Array(corners * 3), msk: new Float32Array(corners) };
    gl.bindVertexArray(ex.vao);
    const attrib = (loc: number, size: number, data: Float32Array): void => {
      const buf = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, buf);
      gl.bufferData(gl.ARRAY_BUFFER, data, gl.DYNAMIC_DRAW);
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, size, gl.FLOAT, false, 0, 0);
      ex.buffers.push(buf);
    };
    this.fillExpanded(m, ex);
    attrib(0, 3, ex.pos);
    attrib(1, 3, ex.nrm);
    attrib(2, 1, ex.msk);
    attrib(3, 3, col);
    gl.bindVertexArray(null);
    return ex;
  }

  private fillExpanded(m: GpuMesh, ex: Expanded): void {
    const { positions, normals, mask } = m.cpu;
    for (let k = 0; k < ex.corners; k++) {
      const v = m.drawn[k];
      ex.pos[k * 3] = positions[v * 3];
      ex.pos[k * 3 + 1] = positions[v * 3 + 1];
      ex.pos[k * 3 + 2] = positions[v * 3 + 2];
      ex.nrm[k * 3] = normals[v * 3];
      ex.nrm[k * 3 + 1] = normals[v * 3 + 1];
      ex.nrm[k * 3 + 2] = normals[v * 3 + 2];
      ex.msk[k] = mask[v];
    }
  }

  private refreshExpanded(m: GpuMesh): void {
    const ex = m.expanded;
    if (!ex) return;
    const gl = this.gl;
    this.fillExpanded(m, ex);
    const [pos, nrm, msk] = ex.buffers;
    gl.bindBuffer(gl.ARRAY_BUFFER, pos);
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, ex.pos);
    gl.bindBuffer(gl.ARRAY_BUFFER, nrm);
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, ex.nrm);
    gl.bindBuffer(gl.ARRAY_BUFFER, msk);
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, ex.msk);
  }

  private dropExpanded(m: GpuMesh): void {
    if (!m.expanded) return;
    m.expanded.buffers.forEach((b) => this.gl.deleteBuffer(b));
    this.gl.deleteVertexArray(m.expanded.vao);
    m.expanded = null;
  }

  private paintBuffer(paint: Float32Array): WebGLBuffer {
    const gl = this.gl;
    const buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, paint, gl.DYNAMIC_DRAW);
    gl.enableVertexAttribArray(3);
    gl.vertexAttribPointer(3, 3, gl.FLOAT, false, 20, 0);
    gl.enableVertexAttribArray(4);
    gl.vertexAttribPointer(4, 2, gl.FLOAT, false, 20, 12);
    return buf;
  }

  private shading = 'clay';
  /** Objects shaded differently from the rest of the view, by id. */
  private perObject = new Map<number, string>();
  /** Image matcaps by name (`img:<file>`), sampled with the view-space normal. */
  private matcaps = new Map<string, WebGLTexture>();

  /** Shades the whole view (clearing per-object choices), or only the object `id` when given. */
  setShading(name: string, id?: number): void {
    if (id === undefined) {
      this.shading = name;
      this.perObject.clear();
    } else this.perObject.set(id, name);
  }

  shadingOf(id: number): string {
    return this.perObject.get(id) ?? this.shading;
  }

  addMatcap(name: string, image: TexImageSource): void {
    const gl = this.gl;
    const old = this.matcaps.get(name);
    if (old) gl.deleteTexture(old);
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    this.matcaps.set(name, tex);
  }

  patch(update: VertexUpdate): boolean {
    const m = this.meshes.get(update.id);
    if (!m || update.indices.length === 0) return false;
    const { cpu } = m;
    let lo = Infinity;
    let hi = -1;
    for (let k = 0; k < update.indices.length; k++) {
      const v = update.indices[k];
      if (v >= cpu.mask.length) continue;
      cpu.positions.set(update.positions.subarray(k * 3, k * 3 + 3), v * 3);
      cpu.normals.set(update.normals.subarray(k * 3, k * 3 + 3), v * 3);
      cpu.mask[v] = update.mask[k];
      if (v < lo) lo = v;
      if (v > hi) hi = v;
    }
    if (hi < 0) return false;
    const gl = this.gl;
    if (update.paint) {
      if (!cpu.paint) {
        const n = cpu.mask.length;
        cpu.paint = new Float32Array(n * 5);
        for (let v = 0; v < n; v++) cpu.paint.set(PAINT_DEFAULT, v * 5);
        for (let k = 0; k < update.indices.length; k++) if (update.indices[k] < n) cpu.paint.set(update.paint.subarray(k * 5, k * 5 + 5), update.indices[k] * 5);
        gl.bindVertexArray(m.vao);
        m.paintBuf = this.paintBuffer(cpu.paint);
        gl.bindVertexArray(null);
        m.buffers.push(m.paintBuf);
      } else {
        for (let k = 0; k < update.indices.length; k++) if (update.indices[k] < cpu.mask.length) cpu.paint.set(update.paint.subarray(k * 5, k * 5 + 5), update.indices[k] * 5);
        gl.bindBuffer(gl.ARRAY_BUFFER, m.paintBuf);
        gl.bufferSubData(gl.ARRAY_BUFFER, lo * 20, cpu.paint, lo * 5, (hi - lo + 1) * 5);
      }
    }
    const [pos, nrm, , msk] = m.buffers;
    gl.bindBuffer(gl.ARRAY_BUFFER, pos);
    gl.bufferSubData(gl.ARRAY_BUFFER, lo * 12, cpu.positions, lo * 3, (hi - lo + 1) * 3);
    gl.bindBuffer(gl.ARRAY_BUFFER, nrm);
    gl.bufferSubData(gl.ARRAY_BUFFER, lo * 12, cpu.normals, lo * 3, (hi - lo + 1) * 3);
    gl.bindBuffer(gl.ARRAY_BUFFER, msk);
    gl.bufferSubData(gl.ARRAY_BUFFER, lo * 4, cpu.mask, lo, hi - lo + 1);
    this.refreshExpanded(m);
    m.revision = update.revision;
    return true;
  }

  setRevision(id: number, revision: number): void {
    const m = this.meshes.get(id);
    if (m) m.revision = revision;
  }

  has(id: number, revision: number): boolean {
    return this.meshes.get(id)?.revision === revision;
  }

  ids(): number[] {
    return [...this.meshes.keys()];
  }

  remove(id: number): void {
    const m = this.meshes.get(id);
    if (!m) return;
    this.dropExpanded(m);
    m.buffers.forEach((b) => this.gl.deleteBuffer(b));
    this.gl.deleteVertexArray(m.vao);
    this.meshes.delete(id);
  }

  render(camera: OrbitCamera, width: number, height: number): void {
    const gl = this.gl;
    const p = this.postSettings;
    const post = this.post && (p.ssao || p.bloom || p.dof || p.vignette || p.grain) ? this.post : null;
    if (post) post.begin(width, height);
    else gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, width, height);
    gl.clearColor(this.clear[0], this.clear[1], this.clear[2], 1);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    const vp = camera.viewProjection();
    const env = this.env && this.lighting.environment ? this.env : null;
    if (env && this.lighting.background) {
      // The environment behind the scene: a fullscreen triangle that looks up each pixel's view direction.
      const inv = invert(vp);
      if (inv) {
        // Drawn at the far plane, which the cleared depth equals, so the depth test is off for it.
        gl.disable(gl.DEPTH_TEST);
        gl.depthMask(false);
        gl.useProgram(this.bgProgram);
        gl.uniformMatrix4fv(gl.getUniformLocation(this.bgProgram, 'uInvViewProj'), false, inv);
        gl.uniform1f(gl.getUniformLocation(this.bgProgram, 'uEnvRot'), this.lighting.rotation);
        gl.uniform1f(gl.getUniformLocation(this.bgProgram, 'uExposure'), this.lighting.exposure);
        gl.uniform1f(gl.getUniformLocation(this.bgProgram, 'uTone'), TONES.indexOf(this.lighting.tone));
        gl.uniform1f(gl.getUniformLocation(this.bgProgram, 'uBlur'), this.lighting.blur * (env.levels - 1));
        gl.activeTexture(gl.TEXTURE1);
        gl.bindTexture(gl.TEXTURE_2D, env.tex);
        gl.uniform1i(gl.getUniformLocation(this.bgProgram, 'uEnv'), 1);
        gl.bindVertexArray(this.bgVao);
        gl.drawArrays(gl.TRIANGLES, 0, 3);
        gl.activeTexture(gl.TEXTURE0);
        gl.depthMask(true);
        gl.enable(gl.DEPTH_TEST);
      }
    }
    gl.useProgram(this.lineProgram);
    gl.uniformMatrix4fv(gl.getUniformLocation(this.lineProgram, 'uViewProj'), false, vp);
    gl.bindVertexArray(this.grid.vao);
    gl.drawArrays(gl.LINES, 0, this.grid.count);
    gl.useProgram(this.meshProgram);
    gl.uniformMatrix4fv(gl.getUniformLocation(this.meshProgram, 'uViewProj'), false, vp);
    gl.uniformMatrix4fv(gl.getUniformLocation(this.meshProgram, 'uView'), false, camera.view());
    gl.uniform1f(gl.getUniformLocation(this.meshProgram, 'uOrtho'), camera.orthographic ? 1 : 0);
    const base = gl.getUniformLocation(this.meshProgram, 'uBase');
    const painted = gl.getUniformLocation(this.meshProgram, 'uPainted');
    const lit = gl.getUniformLocation(this.meshProgram, 'uLit');
    const cap = gl.getUniformLocation(this.meshProgram, 'uCap');
    const tint = gl.getUniformLocation(this.meshProgram, 'uTint');
    const image = gl.getUniformLocation(this.meshProgram, 'uImage');
    gl.uniform1i(gl.getUniformLocation(this.meshProgram, 'uMatcap'), 0);
    gl.uniform1f(gl.getUniformLocation(this.meshProgram, 'uEnvOn'), env ? 1 : 0);
    gl.uniform1f(gl.getUniformLocation(this.meshProgram, 'uEnvRot'), this.lighting.rotation);
    gl.uniform1f(gl.getUniformLocation(this.meshProgram, 'uEnvMaxLod'), env ? env.levels - 1 : 0);
    gl.uniform1f(gl.getUniformLocation(this.meshProgram, 'uExposure'), this.lighting.exposure);
    gl.uniform1f(gl.getUniformLocation(this.meshProgram, 'uLightScale'), env ? this.lighting.lights : 1);
    gl.uniform1f(gl.getUniformLocation(this.meshProgram, 'uTone'), TONES.indexOf(this.lighting.tone));
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, env ? env.tex : this.blankEnv);
    gl.uniform1i(gl.getUniformLocation(this.meshProgram, 'uEnv'), 1);
    gl.activeTexture(gl.TEXTURE0);
    const model = gl.getUniformLocation(this.meshProgram, 'uModel');
    for (const [id, m] of this.meshes) {
      gl.uniformMatrix4fv(model, false, this.previews.get(id) ?? IDENTITY);
      const name = this.shadingOf(id);
      const tex = this.matcaps.get(name);
      const s = SHADINGS[name] ?? SHADINGS.clay;
      gl.uniform1f(lit, tex ? 0 : s.lit ? 1 : 0);
      gl.uniform4f(cap, ...s.cap);
      gl.uniform3f(tint, ...s.tint);
      gl.uniform1f(image, tex ? 1 : 0);
      gl.bindTexture(gl.TEXTURE_2D, tex ?? this.blank);
      if (id === this.active) gl.uniform3f(base, 0.86, 0.74, 0.66);
      else gl.uniform3f(base, 0.72, 0.7, 0.68);
      if (this.showGroups && m.expanded) {
        gl.uniform1f(painted, 1);
        gl.bindVertexArray(m.expanded.vao);
        gl.vertexAttrib2f(4, PAINT_DEFAULT[3], PAINT_DEFAULT[4]);
        gl.drawArrays(gl.TRIANGLES, 0, m.expanded.corners);
        continue;
      }
      gl.uniform1f(painted, m.paintBuf ? 1 : 0);
      gl.bindVertexArray(m.vao);
      if (!m.paintBuf) {
        gl.vertexAttrib3f(3, PAINT_DEFAULT[0], PAINT_DEFAULT[1], PAINT_DEFAULT[2]);
        gl.vertexAttrib2f(4, PAINT_DEFAULT[3], PAINT_DEFAULT[4]);
      }
      gl.drawElements(gl.TRIANGLES, m.count, gl.UNSIGNED_INT, 0);
    }
    gl.bindVertexArray(null);
    if (post) {
      const proj = camera.projection();
      const inv = invert(proj);
      if (inv) post.finish(p, proj, inv, camera.distance * 0.1, camera.distance);
    }
  }

  private post: PostFx | null = null;
  private postSettings: PostSettings = { ...NO_POST };

  /** Turns post effects on or off; the offscreen pipeline is built on first use. */
  setPost(settings: PostSettings): void {
    this.postSettings = { ...settings };
    if (!this.post && (settings.ssao || settings.bloom || settings.dof || settings.vignette || settings.grain)) this.post = new PostFx(this.gl);
  }

  private env: { tex: WebGLTexture; levels: number } | null = null;
  lighting: Lighting = { environment: false, background: false, rotation: 0, exposure: 1, lights: 0.35, tone: 'reinhard', blur: 0.35 };

  /** Uploads an environment as a float texture with its own mip chain (null removes it). */
  setEnvironment(img: EnvImage | null): void {
    const gl = this.gl;
    if (this.env) gl.deleteTexture(this.env.tex);
    this.env = null;
    if (!img) return;
    const chain = mipChain(img);
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    chain.forEach((level, i) => gl.texImage2D(gl.TEXTURE_2D, i, gl.RGB16F, level.width, level.height, 0, gl.RGB, gl.FLOAT, level.rgb));
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAX_LEVEL, chain.length - 1);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    this.env = { tex, levels: chain.length };
  }
}

export const TONES = ['reinhard', 'aces', 'none'];

const IDENTITY = new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]);

export interface Lighting {
  /** Light lit (PBR) shading with the environment image. */
  environment: boolean;
  /** Show the environment behind the model. */
  background: boolean;
  /** Turn of the environment about the vertical, in radians. */
  rotation: number;
  exposure: number;
  /** The three fixed lights' share when an environment lights the scene. */
  lights: number;
  tone: string;
  /** How blurred the background is, 0 sharp to 1 fully blurred. */
  blur: number;
}

const BG_VS = `#version 300 es
out vec2 vNdc;
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2)) * 2.0 - 1.0;
  vNdc = p;
  gl_Position = vec4(p, 1.0, 1.0);
}`;

const BG_FS = `#version 300 es
precision highp float;
in vec2 vNdc;
uniform mat4 uInvViewProj;
uniform sampler2D uEnv;
uniform float uEnvRot;
uniform float uExposure;
uniform float uTone;
uniform float uBlur;
out vec4 outColor;
const float PI_ = 3.14159265;
void main() {
  vec4 a = uInvViewProj * vec4(vNdc, -1.0, 1.0);
  vec4 b = uInvViewProj * vec4(vNdc, 1.0, 1.0);
  vec3 d = normalize(b.xyz / b.w - a.xyz / a.w);
  float az = atan(d.x, d.z) + uEnvRot;
  vec3 c = textureLod(uEnv, vec2(fract((az + PI_) / (2.0 * PI_)), acos(clamp(d.y, -1.0, 1.0)) / PI_), uBlur).rgb * uExposure;
  if (uTone > 1.5) c = clamp(c, 0.0, 1.0);
  else if (uTone > 0.5) c = clamp((c * (2.51 * c + 0.03)) / (c * (2.43 * c + 0.59) + 0.14), 0.0, 1.0);
  else c = c / (c + vec3(1.0));
  outColor = vec4(pow(c, vec3(1.0 / 2.2)), 1.0);
}`;
