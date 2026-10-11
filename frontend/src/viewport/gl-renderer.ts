import type { MeshBuffers, VertexUpdate } from '../engine/service';
import type { OrbitCamera } from './camera';

const MESH_VS = `#version 300 es
layout(location=0) in vec3 aPos;
layout(location=1) in vec3 aNrm;
layout(location=2) in float aMask;
layout(location=3) in vec3 aCol;
layout(location=4) in vec2 aMat;
uniform mat4 uViewProj;
uniform mat4 uView;
out vec3 vNrm;
out vec3 vViewPos;
out float vMask;
out vec3 vCol;
out vec2 vMat;
void main() {
  vMask = aMask;
  vCol = aCol;
  vMat = aMat;
  vNrm = mat3(uView) * aNrm;
  vViewPos = (uView * vec4(aPos, 1.0)).xyz;
  gl_Position = uViewProj * vec4(aPos, 1.0);
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
    vec3 sky = mix(vec3(0.16, 0.15, 0.14), vec3(0.42, 0.45, 0.5), n.y * 0.5 + 0.5);
    vec3 f0 = mix(vec3(0.04), albedo, metal);
    col += sky * (albedo * (1.0 - metal) + f0 * (1.0 - rough * 0.7));
    col = col / (col + vec3(1.0));
    col = pow(col, vec3(1.0 / 2.2));
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
  private clear: [number, number, number] = [0.11, 0.106, 0.114];
  active: number | null = null;

  constructor(canvas: HTMLCanvasElement) {
    const gl = canvas.getContext('webgl2', { antialias: true, alpha: false, preserveDrawingBuffer: false });
    if (!gl) throw new Error('WebGL2 is not available');
    this.gl = gl;
    this.meshProgram = compile(gl, MESH_VS, MESH_FS);
    this.lineProgram = compile(gl, LINE_VS, LINE_FS);
    this.grid = this.buildGrid(20, 1);
    gl.enable(gl.DEPTH_TEST);
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

  upload(mesh: MeshBuffers): void {
    const gl = this.gl;
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
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, idx);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, mesh.indices, gl.STATIC_DRAW);
    const paint = mesh.paint ? new Float32Array(mesh.paint) : null;
    const paintBuf = paint ? this.paintBuffer(paint) : null;
    gl.bindVertexArray(null);
    const cpu: MeshBuffers = { ...mesh, positions: new Float32Array(mesh.positions), normals: new Float32Array(mesh.normals), mask: new Float32Array(mesh.mask), paint };
    this.meshes.set(mesh.id, { vao, buffers: paintBuf ? [pos, nrm, idx, msk, paintBuf] : [pos, nrm, idx, msk], count: mesh.indices.length, revision: mesh.revision, cpu, paintBuf });
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

  private shading = SHADINGS.clay;

  setShading(name: string): void {
    this.shading = SHADINGS[name] ?? SHADINGS.clay;
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
    m.buffers.forEach((b) => this.gl.deleteBuffer(b));
    this.gl.deleteVertexArray(m.vao);
    this.meshes.delete(id);
  }

  render(camera: OrbitCamera, width: number, height: number): void {
    const gl = this.gl;
    gl.viewport(0, 0, width, height);
    gl.clearColor(this.clear[0], this.clear[1], this.clear[2], 1);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    const vp = camera.viewProjection();
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
    const s = this.shading;
    gl.uniform1f(gl.getUniformLocation(this.meshProgram, 'uLit'), s.lit ? 1 : 0);
    gl.uniform4f(gl.getUniformLocation(this.meshProgram, 'uCap'), ...s.cap);
    gl.uniform3f(gl.getUniformLocation(this.meshProgram, 'uTint'), ...s.tint);
    for (const [id, m] of this.meshes) {
      if (id === this.active) gl.uniform3f(base, 0.86, 0.74, 0.66);
      else gl.uniform3f(base, 0.72, 0.7, 0.68);
      gl.uniform1f(painted, m.paintBuf ? 1 : 0);
      gl.bindVertexArray(m.vao);
      if (!m.paintBuf) {
        gl.vertexAttrib3f(3, PAINT_DEFAULT[0], PAINT_DEFAULT[1], PAINT_DEFAULT[2]);
        gl.vertexAttrib2f(4, PAINT_DEFAULT[3], PAINT_DEFAULT[4]);
      }
      gl.drawElements(gl.TRIANGLES, m.count, gl.UNSIGNED_INT, 0);
    }
    gl.bindVertexArray(null);
  }
}
