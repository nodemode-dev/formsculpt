import type { MeshBuffers, VertexUpdate } from '../engine/service';
import type { OrbitCamera } from './camera';

const MESH_VS = `#version 300 es
layout(location=0) in vec3 aPos;
layout(location=1) in vec3 aNrm;
layout(location=2) in float aMask;
uniform mat4 uViewProj;
uniform mat4 uView;
out vec3 vNrm;
out vec3 vViewPos;
out float vMask;
void main() {
  vMask = aMask;
  vNrm = mat3(uView) * aNrm;
  vViewPos = (uView * vec4(aPos, 1.0)).xyz;
  gl_Position = uViewProj * vec4(aPos, 1.0);
}`;

const MESH_FS = `#version 300 es
precision highp float;
in vec3 vNrm;
in vec3 vViewPos;
in float vMask;
uniform vec3 uBase;
uniform float uOrtho;
out vec4 outColor;
void main() {
  vec3 n = normalize(vNrm);
  vec3 v = uOrtho > 0.5 ? vec3(0.0, 0.0, 1.0) : normalize(-vViewPos);
  if (dot(n, v) < 0.0) n = -n;
  vec3 r = reflect(-v, n);
  vec2 m = r.xy / (2.0 * sqrt(r.x * r.x + r.y * r.y + (r.z + 1.0) * (r.z + 1.0))) + 0.5;
  float key = clamp(dot(n, normalize(vec3(-0.45, 0.65, 0.6))), 0.0, 1.0);
  float fill = clamp(dot(n, normalize(vec3(0.6, 0.1, 0.5))), 0.0, 1.0) * 0.25;
  float rim = pow(1.0 - clamp(dot(n, v), 0.0, 1.0), 3.0) * 0.35;
  float spec = pow(clamp(1.0 - distance(m, vec2(0.34, 0.68)) * 2.2, 0.0, 1.0), 6.0) * 0.45;
  float cavity = mix(0.55, 1.0, smoothstep(-0.2, 0.9, n.y * 0.5 + 0.5));
  vec3 col = uBase * (0.18 + key * 0.75 + fill) * cavity + vec3(rim + spec);
  col = mix(col, col * vec3(0.32, 0.34, 0.4), clamp(vMask, 0.0, 1.0) * 0.85);
  outColor = vec4(pow(col, vec3(1.0 / 1.1)), 1.0);
}`;

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
    gl.bindVertexArray(null);
    const cpu: MeshBuffers = { ...mesh, positions: new Float32Array(mesh.positions), normals: new Float32Array(mesh.normals), mask: new Float32Array(mesh.mask) };
    this.meshes.set(mesh.id, { vao, buffers: [pos, nrm, idx, msk], count: mesh.indices.length, revision: mesh.revision, cpu });
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
    for (const [id, m] of this.meshes) {
      if (id === this.active) gl.uniform3f(base, 0.86, 0.74, 0.66);
      else gl.uniform3f(base, 0.72, 0.7, 0.68);
      gl.bindVertexArray(m.vao);
      gl.drawElements(gl.TRIANGLES, m.count, gl.UNSIGNED_INT, 0);
    }
    gl.bindVertexArray(null);
  }
}
