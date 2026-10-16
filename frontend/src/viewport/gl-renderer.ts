import { PAINT_FLOATS, type MeshBuffers, type VertexUpdate } from '../engine/service';
import type { OrbitCamera } from './camera';
import { mipChain, type EnvImage } from './environment';
import { invert, lookAt, multiply, ortho } from './math';
import { NO_POST, PostFx, type PostSettings } from './post';

const MESH_VS = `#version 300 es
layout(location=0) in vec3 aPos;
layout(location=1) in vec3 aNrm;
layout(location=2) in float aMask;
layout(location=3) in vec3 aCol;
layout(location=4) in vec2 aMat;
layout(location=5) in vec2 aFx;
uniform mat4 uViewProj;
uniform mat4 uView;
uniform mat4 uModel;
out vec3 vNrm;
out vec3 vViewPos;
out vec3 vWorld;
out float vMask;
out vec3 vCol;
out vec2 vMat;
out vec2 vFx;
void main() {
  vMask = aMask;
  vCol = aCol;
  vMat = aMat;
  vFx = aFx;
  // uModel is the identity except while the transform gizmo previews a move, turn or scale.
  vec4 world = uModel * vec4(aPos, 1.0);
  vNrm = mat3(uView) * (transpose(inverse(mat3(uModel))) * aNrm);
  vViewPos = (uView * world).xyz;
  vWorld = world.xyz;
  gl_Position = uViewProj * world;
}`;

const MESH_FS = `#version 300 es
precision highp float;
in vec3 vNrm;
in vec3 vViewPos;
in vec3 vWorld;
uniform vec3 uKeyDir;
uniform vec3 uKeyColor;
uniform float uShadowOn;
uniform mat4 uShadowVP;
uniform sampler2D uShadowMap;
uniform float uShadowBias;
// Added lights beyond the key (view-space directions, linear colours times strength); they cast no shadows.
uniform int uExtraCount;
uniform vec3 uExtraDir[3];
uniform vec3 uExtraColor[3];
// The key light's visibility from its shadow map: a 3 by 3 percentage-closer filter for a soft edge.
float keyShadow() {
  if (uShadowOn < 0.5) return 1.0;
  vec4 c = uShadowVP * vec4(vWorld, 1.0);
  vec3 q = c.xyz / c.w * 0.5 + 0.5;
  if (q.x < 0.0 || q.y < 0.0 || q.x > 1.0 || q.y > 1.0 || q.z > 1.0) return 1.0;
  vec2 texel = 1.0 / vec2(textureSize(uShadowMap, 0));
  float lit = 0.0;
  for (int x = -1; x <= 1; x++)
    for (int y = -1; y <= 1; y++) lit += q.z - uShadowBias <= texture(uShadowMap, q.xy + vec2(x, y) * texel).r ? 1.0 : 0.0;
  return mix(0.25, 1.0, lit / 9.0);
}
uniform float uDepthOn;
uniform float uShadowRange;
uniform float uSss;
uniform vec3 uSssColor;
uniform float uSssRadius;
uniform float uTrans;
uniform float uIor;
// How much material lies between this point and the key light, in world units, from the light's depth map; -1 when
// there is no map.
float thickness() {
  if (uDepthOn < 0.5) return -1.0;
  vec4 c = uShadowVP * vec4(vWorld, 1.0);
  vec3 q = c.xyz / c.w * 0.5 + 0.5;
  if (q.x < 0.0 || q.y < 0.0 || q.x > 1.0 || q.y > 1.0 || q.z > 1.0) return -1.0;
  return max(q.z - texture(uShadowMap, q.xy).r - uShadowBias, 0.0) * uShadowRange;
}
in float vMask;
in vec3 vCol;
in vec2 vMat;
// Painted opacity (x) and emissive (y).
in vec2 vFx;
uniform vec3 uBase;
uniform float uOrtho;
uniform float uPainted;
uniform float uChecker;
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
  // Painted opacity draws as a screen-door: a fixed noise per pixel decides which fragments stay, so see-through
  // parts need no sorting and keep writing depth where they are drawn.
  if (uPainted > 0.5 && vFx.x < 0.999) {
    float gate = fract(52.9829189 * fract(dot(gl_FragCoord.xy, vec2(0.06711056, 0.00583715))));
    if (vFx.x <= gate) discard;
  }
  vec3 n = normalize(vNrm);
  vec3 v = uOrtho > 0.5 ? vec3(0.0, 0.0, 1.0) : normalize(-vViewPos);
  if (dot(n, v) < 0.0) n = -n;
  vec3 albedo = uPainted > 0.5 ? vCol : uBase;
  if (uChecker > 0.5) {
    vec2 cell = floor(fract(vCol.xy) * 16.0);
    albedo = mod(cell.x + cell.y, 2.0) < 0.5 ? vec3(0.86, 0.86, 0.84) : vec3(0.18, 0.2, 0.24);
    float tile = floor(vCol.x) + 10.0 * floor(vCol.y);
    if (tile > 0.5) albedo *= mix(vec3(1.0), 0.6 + 0.4 * cos(6.2832 * (tile * 0.17 + vec3(0.0, 0.33, 0.67))), 0.6);
    if (vCol.x < 0.0 || vCol.x >= 10.0 || vCol.y < 0.0) albedo = vec3(0.85, 0.25, 0.2);
  }
  vec3 col;
  if (uLit > 0.5) {
    float rough = clamp(vMat.x, 0.05, 1.0);
    float metal = clamp(vMat.y, 0.0, 1.0);
    col = light(n, v, uKeyDir, uKeyColor, albedo, rough, metal) * keyShadow();
    for (int i = 0; i < 3; i++) if (i < uExtraCount) col += light(n, v, uExtraDir[i], uExtraColor[i], albedo, rough, metal);
    col += light(n, v, normalize(vec3(0.6, 0.1, 0.5)), vec3(0.55, 0.62, 0.75), albedo, rough, metal);
    col += light(n, v, normalize(vec3(0.0, 0.35, -1.0)), vec3(0.9), albedo, rough, metal);
    if (uSss > 0.0) {
      // Subsurface: key light wraps past the terminator tinted, and passes through parts thinner than the radius when
      // lit from behind (thickness from the light's depth map; a view-angle guess without one).
      float ndl = dot(n, uKeyDir);
      float w = 0.6 * uSss;
      float wrap = max((ndl + w) / (1.0 + w), 0.0) - max(ndl, 0.0);
      float t = thickness();
      float through = t >= 0.0 ? exp(-t / max(uSssRadius, 1e-4)) : pow(clamp(dot(v, -uKeyDir), 0.0, 1.0), 3.0) * 0.5;
      // Only surfaces turned away from the light pass it through, fading in past the terminator so there is no seam.
      through *= smoothstep(0.0, 0.6, -ndl);
      col = mix(col, col * mix(vec3(1.0), uSssColor, 0.35), uSss * 0.5);
      col += (uSssColor * albedo * wrap / PI + uSssColor * through * 0.35) * uKeyColor * uSss;
    }
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
    if (uTrans > 0.0) {
      // Transmission: what lies behind (the environment, or the sky without one) refracted through the surface at the
      // index of refraction and tinted by its colour, with the Fresnel reflection kept on top.
      float ft = pow((uIor - 1.0) / (uIor + 1.0), 2.0);
      float fres = ft + (1.0 - ft) * pow(1.0 - max(dot(n, v), 0.0), 5.0);
      vec3 wd = transpose(mat3(uView)) * refract(-v, n, 1.0 / uIor);
      vec3 behind = uEnvOn > 0.5 ? envAt(wd, rough * (uEnvMaxLod - 2.0)) : mix(vec3(0.16, 0.15, 0.14), vec3(0.42, 0.45, 0.5), wd.y * 0.5 + 0.5) * 1.6;
      col = mix(col, behind * albedo + col * fres, uTrans * (1.0 - fres));
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
    float key = clamp(dot(n, uKeyDir), 0.0, 1.0) * keyShadow() * dot(uKeyColor, vec3(0.1282));
    float fill = clamp(dot(n, normalize(vec3(0.6, 0.1, 0.5))), 0.0, 1.0) * 0.25;
    for (int i = 0; i < 3; i++) if (i < uExtraCount) fill += clamp(dot(n, uExtraDir[i]), 0.0, 1.0) * dot(uExtraColor[i], vec3(0.1282)) * uCap.x;
    float rim = pow(1.0 - clamp(dot(n, v), 0.0, 1.0), 3.0) * uCap.w;
    float spec = pow(clamp(1.0 - distance(m, vec2(0.34, 0.68)) * 2.2, 0.0, 1.0), uCap.z) * uCap.y;
    float cavity = mix(0.55, 1.0, smoothstep(-0.2, 0.9, n.y * 0.5 + 0.5));
    vec3 base = albedo * uTint;
    col = base * (0.18 + key * uCap.x + fill) * cavity + (vec3(rim) + spec * mix(vec3(1.0), uTint, 0.6));
    col = pow(col, vec3(1.0 / 1.1));
  }
  // Emissive paint glows in its own colour whatever the light or matcap.
  if (uPainted > 0.5 && vFx.y > 0.0) col = mix(col, pow(albedo, vec3(1.0 / 2.2)) * 1.15, clamp(vFx.y, 0.0, 1.0));
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

/** Colour, roughness, metalness, opacity, emissive: the engine's default paint. */
const PAINT_DEFAULT = [0.8, 0.8, 0.8, 0.5, 0.0, 1.0, 0.0];
const PF = PAINT_FLOATS;

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
  /** A per-corner copy carrying UVs for the checker view, or null. */
  checker: Expanded | null;
  /** Bounds at upload, for fitting the shadow map (strokes move vertices only a little). */
  lo: [number, number, number];
  hi: [number, number, number];
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
  private meshes = new Map<number, GpuMesh>();
  private gradientProgram: WebGLProgram;
  private floorProgram: WebGLProgram;
  backdrop: Backdrop = { mode: 'gradient', top: [0.235, 0.235, 0.255], bottom: [0.09, 0.09, 0.1], mid: 0.55, vignette: 0.35, grid: true, gridColor: [0.62, 0.62, 0.66] };
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
    this.bgProgram = compile(gl, BG_VS, BG_FS);
    this.gradientProgram = compile(gl, BG_VS, GRADIENT_FS);
    this.floorProgram = compile(gl, BG_VS, FLOOR_FS);
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

  /** The studio gradient behind the scene, drawn at the far plane with the depth test off. */
  private drawGradient(width: number, height: number): void {
    const gl = this.gl;
    const b = this.backdrop;
    const p = this.gradientProgram;
    gl.disable(gl.DEPTH_TEST);
    gl.depthMask(false);
    gl.useProgram(p);
    gl.uniform3fv(gl.getUniformLocation(p, 'uTop'), b.top);
    gl.uniform3fv(gl.getUniformLocation(p, 'uBottom'), b.bottom);
    gl.uniform1f(gl.getUniformLocation(p, 'uMid'), b.mid);
    gl.uniform1f(gl.getUniformLocation(p, 'uVignette'), b.vignette);
    gl.uniform2f(gl.getUniformLocation(p, 'uSize'), width, height);
    gl.bindVertexArray(this.bgVao);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    gl.depthMask(true);
    gl.enable(gl.DEPTH_TEST);
  }

  /** The floor grid: an infinite plane at y = 0 traced per pixel, lines that stay one pixel wide at any distance, a
   *  scale that follows the camera distance (blending between powers of ten), fading with distance and at grazing
   *  angles so a side view shows no line clutter; the x axis red, the z axis blue. Drawn behind the meshes. */
  private drawFloor(camera: OrbitCamera, vp: Float32Array): void {
    const inv = invert(vp);
    if (!inv) return;
    const gl = this.gl;
    const p = this.floorProgram;
    const eye = camera.eye();
    const t = camera.target;
    const dir = [t[0] - eye[0], t[1] - eye[1], t[2] - eye[2]];
    const len = Math.hypot(dir[0], dir[1], dir[2]) || 1;
    const level = Math.log10(Math.max(camera.distance * 0.6, 1e-4));
    const base = Math.floor(level);
    gl.disable(gl.DEPTH_TEST);
    gl.depthMask(false);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    gl.useProgram(p);
    gl.uniformMatrix4fv(gl.getUniformLocation(p, 'uInvViewProj'), false, inv);
    gl.uniform3f(gl.getUniformLocation(p, 'uEye'), eye[0], eye[1], eye[2]);
    gl.uniform3f(gl.getUniformLocation(p, 'uViewDir'), dir[0] / len, dir[1] / len, dir[2] / len);
    gl.uniform1f(gl.getUniformLocation(p, 'uOrtho'), camera.orthographic ? 1 : 0);
    gl.uniform1f(gl.getUniformLocation(p, 'uScale'), 10 ** base);
    gl.uniform1f(gl.getUniformLocation(p, 'uFrac'), level - base);
    gl.uniform1f(gl.getUniformLocation(p, 'uFade'), camera.distance * (camera.orthographic ? 12 : 7));
    gl.uniform3fv(gl.getUniformLocation(p, 'uLine'), this.backdrop.gridColor);
    gl.uniform1f(gl.getUniformLocation(p, 'uAlpha'), 0.32);
    gl.bindVertexArray(this.bgVao);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    gl.disable(gl.BLEND);
    gl.depthMask(true);
    gl.enable(gl.DEPTH_TEST);
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
    const lo: [number, number, number] = [Infinity, Infinity, Infinity];
    const hi: [number, number, number] = [-Infinity, -Infinity, -Infinity];
    for (let i = 0; i < cpu.positions.length; i += 3) {
      for (let c = 0; c < 3; c++) {
        const x = cpu.positions[i + c];
        if (x < lo[c]) lo[c] = x;
        if (x > hi[c]) hi[c] = x;
      }
    }
    const gpu: GpuMesh = { vao, buffers: paintBuf ? [pos, nrm, idx, msk, paintBuf] : [pos, nrm, idx, msk], count: drawn.length, revision: mesh.revision, cpu, paintBuf, drawn, drawnTris, expanded: null, checker: null, lo, hi };
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

  private expand(m: GpuMesh, given?: Float32Array): Expanded {
    const gl = this.gl;
    const corners = m.drawn.length;
    const col = given ?? new Float32Array(corners * 3);
    if (!given) {
      const groups = m.cpu.groups as Uint32Array;
      for (let k = 0; k < m.drawnTris.length; k++) {
        const c = groupColor(groups[m.drawnTris[k]]);
        for (let j = 0; j < 3; j++) col.set(c, (k * 3 + j) * 3);
      }
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
    const gl = this.gl;
    for (const ex of [m.expanded, m.checker]) {
      if (!ex) continue;
      this.fillExpanded(m, ex);
      const [pos, nrm, msk] = ex.buffers;
      gl.bindBuffer(gl.ARRAY_BUFFER, pos);
      gl.bufferSubData(gl.ARRAY_BUFFER, 0, ex.pos);
      gl.bindBuffer(gl.ARRAY_BUFFER, nrm);
      gl.bufferSubData(gl.ARRAY_BUFFER, 0, ex.nrm);
      gl.bindBuffer(gl.ARRAY_BUFFER, msk);
      gl.bufferSubData(gl.ARRAY_BUFFER, 0, ex.msk);
    }
  }

  private dropExpanded(m: GpuMesh): void {
    if (m.checker) this.dropChecker(m);
    if (!m.expanded) return;
    m.expanded.buffers.forEach((b) => this.gl.deleteBuffer(b));
    this.gl.deleteVertexArray(m.expanded.vao);
    m.expanded = null;
  }

  private dropChecker(m: GpuMesh): void {
    if (!m.checker) return;
    m.checker.buffers.forEach((b) => this.gl.deleteBuffer(b));
    this.gl.deleteVertexArray(m.checker.vao);
    m.checker = null;
  }

  /**
   * UV checker on one object: per-corner UVs (two floats per triangle corner, in index order, as get_uvs returns them)
   * ride in the colour attribute of a per-corner copy and the fragment shader draws a 16 by 16 checker from them.
   * Null turns it off. Only the revision the UVs came from is drawn; a new upload drops the checker.
   */
  setChecker(id: number, uvs: Float32Array | null): boolean {
    const m = this.meshes.get(id);
    if (!m) return false;
    this.dropChecker(m);
    if (!uvs) return true;
    if (uvs.length < m.drawnTris.length * 6) return false;
    const col = new Float32Array(m.drawn.length * 3);
    for (let k = 0; k < m.drawnTris.length; k++) {
      const tri = m.drawnTris[k];
      for (let j = 0; j < 3; j++) {
        col[(k * 3 + j) * 3] = uvs[(tri * 3 + j) * 2];
        col[(k * 3 + j) * 3 + 1] = uvs[(tri * 3 + j) * 2 + 1];
      }
    }
    m.checker = this.expand(m, col);
    return true;
  }

  private paintBuffer(paint: Float32Array): WebGLBuffer {
    const gl = this.gl;
    const buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, paint, gl.DYNAMIC_DRAW);
    gl.enableVertexAttribArray(3);
    gl.vertexAttribPointer(3, 3, gl.FLOAT, false, PF * 4, 0);
    gl.enableVertexAttribArray(4);
    gl.vertexAttribPointer(4, 2, gl.FLOAT, false, PF * 4, 12);
    gl.enableVertexAttribArray(5);
    gl.vertexAttribPointer(5, 2, gl.FLOAT, false, PF * 4, 20);
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

  private materials = new Map<number, ObjectMaterial>();

  /** An object's subsurface and transmission, as the engine stores them (null clears it to opaque). Returns whether
   * the drawing changes, so the caller redraws only then. */
  setMaterial(id: number, m: ObjectMaterial | null): boolean {
    const was = this.materials.get(id);
    if (m && (m.subsurface > 0 || m.transmission > 0)) {
      if (was && JSON.stringify(was) === JSON.stringify(m)) return false;
      this.materials.set(id, m);
      return true;
    }
    return this.materials.delete(id);
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
        cpu.paint = new Float32Array(n * PF);
        for (let v = 0; v < n; v++) cpu.paint.set(PAINT_DEFAULT, v * PF);
        for (let k = 0; k < update.indices.length; k++) if (update.indices[k] < n) cpu.paint.set(update.paint.subarray(k * PF, k * PF + PF), update.indices[k] * PF);
        gl.bindVertexArray(m.vao);
        m.paintBuf = this.paintBuffer(cpu.paint);
        gl.bindVertexArray(null);
        m.buffers.push(m.paintBuf);
      } else {
        for (let k = 0; k < update.indices.length; k++) if (update.indices[k] < cpu.mask.length) cpu.paint.set(update.paint.subarray(k * PF, k * PF + PF), update.indices[k] * PF);
        gl.bindBuffer(gl.ARRAY_BUFFER, m.paintBuf);
        gl.bufferSubData(gl.ARRAY_BUFFER, lo * PF * 4, cpu.paint, lo * PF, (hi - lo + 1) * PF);
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
    // The key light: its world direction (from azimuth and elevation, about the view when it follows the camera).
    const L = this.lighting;
    const az = (L.keyAzimuth * Math.PI) / 180;
    const el = (L.keyElevation * Math.PI) / 180;
    const dir: [number, number, number] = [Math.cos(el) * Math.sin(az), Math.sin(el), Math.cos(el) * Math.cos(az)];
    const view = camera.view();
    // view is column-major: world to view rotates by its upper 3 by 3; its transpose takes view back to world.
    const toView = (d: [number, number, number]): [number, number, number] => [view[0] * d[0] + view[4] * d[1] + view[8] * d[2], view[1] * d[0] + view[5] * d[1] + view[9] * d[2], view[2] * d[0] + view[6] * d[1] + view[10] * d[2]];
    const toWorld = (d: [number, number, number]): [number, number, number] => [view[0] * d[0] + view[1] * d[1] + view[2] * d[2], view[4] * d[0] + view[5] * d[1] + view[6] * d[2], view[8] * d[0] + view[9] * d[1] + view[10] * d[2]];
    const keyView = L.keyFollowsView ? dir : toView(dir);
    const keyWorld = L.keyFollowsView ? toWorld(dir) : dir;
    // Subsurface reads thickness from the key light's depth map, so the map is drawn for it too.
    const subsurface = [...this.materials.entries()].some(([id, m]) => m.subsurface > 0 && this.meshes.has(id));
    const shadowMap = (L.shadows || L.ground || subsurface) && this.meshes.size > 0 ? this.renderShadow(keyWorld) : null;
    const shadowVP = L.shadows ? shadowMap : null;
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
    } else if (this.backdrop.mode === 'gradient') {
      this.drawGradient(width, height);
    }
    if (this.backdrop.grid) this.drawFloor(camera, vp);
    gl.useProgram(this.meshProgram);
    gl.uniform1f(gl.getUniformLocation(this.meshProgram, 'uChecker'), 0);
    gl.uniformMatrix4fv(gl.getUniformLocation(this.meshProgram, 'uViewProj'), false, vp);
    gl.uniformMatrix4fv(gl.getUniformLocation(this.meshProgram, 'uView'), false, camera.view());
    gl.uniform1f(gl.getUniformLocation(this.meshProgram, 'uOrtho'), camera.orthographic ? 1 : 0);
    const base = gl.getUniformLocation(this.meshProgram, 'uBase');
    const painted = gl.getUniformLocation(this.meshProgram, 'uPainted');
    const checker = gl.getUniformLocation(this.meshProgram, 'uChecker');
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
    const k = L.keyIntensity;
    gl.uniform3f(gl.getUniformLocation(this.meshProgram, 'uKeyDir'), ...keyView);
    gl.uniform3f(gl.getUniformLocation(this.meshProgram, 'uKeyColor'), 2.6 * k * L.keyColor[0], 2.6 * k * L.keyColor[1], 2.6 * k * L.keyColor[2]);
    gl.uniform1f(gl.getUniformLocation(this.meshProgram, 'uShadowOn'), shadowVP ? 1 : 0);
    gl.uniform1f(gl.getUniformLocation(this.meshProgram, 'uDepthOn'), shadowMap ? 1 : 0);
    gl.uniform1f(gl.getUniformLocation(this.meshProgram, 'uShadowRange'), this.shadowRange);
    if (shadowMap && !shadowVP) gl.uniformMatrix4fv(gl.getUniformLocation(this.meshProgram, 'uShadowVP'), false, shadowMap);
    gl.uniform1f(gl.getUniformLocation(this.meshProgram, 'uShadowBias'), 0.003);
    const extra = L.extra.slice(0, 3);
    const dirs = new Float32Array(9);
    const cols = new Float32Array(9);
    extra.forEach((x, i) => {
      const a = (x.azimuth * Math.PI) / 180;
      const e = (x.elevation * Math.PI) / 180;
      const d: [number, number, number] = [Math.cos(e) * Math.sin(a), Math.sin(e), Math.cos(e) * Math.cos(a)];
      dirs.set(x.followsView ? d : toView(d), i * 3);
      cols.set([2.6 * x.intensity * x.color[0], 2.6 * x.intensity * x.color[1], 2.6 * x.intensity * x.color[2]], i * 3);
    });
    gl.uniform1i(gl.getUniformLocation(this.meshProgram, 'uExtraCount'), extra.length);
    gl.uniform3fv(gl.getUniformLocation(this.meshProgram, 'uExtraDir'), dirs);
    gl.uniform3fv(gl.getUniformLocation(this.meshProgram, 'uExtraColor'), cols);
    if (shadowVP) gl.uniformMatrix4fv(gl.getUniformLocation(this.meshProgram, 'uShadowVP'), false, shadowVP);
    gl.activeTexture(gl.TEXTURE2);
    gl.bindTexture(gl.TEXTURE_2D, shadowMap && this.shadow ? this.shadow.tex : this.blankEnv);
    gl.uniform1i(gl.getUniformLocation(this.meshProgram, 'uShadowMap'), 2);
    gl.activeTexture(gl.TEXTURE0);
    const model = gl.getUniformLocation(this.meshProgram, 'uModel');
    const sss = gl.getUniformLocation(this.meshProgram, 'uSss');
    const sssColor = gl.getUniformLocation(this.meshProgram, 'uSssColor');
    const sssRadius = gl.getUniformLocation(this.meshProgram, 'uSssRadius');
    const trans = gl.getUniformLocation(this.meshProgram, 'uTrans');
    const ior = gl.getUniformLocation(this.meshProgram, 'uIor');
    for (const [id, m] of this.meshes) {
      gl.uniformMatrix4fv(model, false, this.previews.get(id) ?? IDENTITY);
      const mat = this.materials.get(id);
      gl.uniform1f(sss, mat?.subsurface ?? 0);
      gl.uniform3f(sssColor, ...(mat?.subsurfaceColor ?? [1, 0.35, 0.25]));
      gl.uniform1f(sssRadius, mat?.subsurfaceRadius ?? 0.2);
      gl.uniform1f(trans, mat?.transmission ?? 0);
      gl.uniform1f(ior, mat?.ior ?? 1.45);
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
      if (m.checker) {
        gl.uniform1f(painted, 1);
        gl.uniform1f(checker, 1);
        gl.bindVertexArray(m.checker.vao);
        gl.vertexAttrib2f(4, PAINT_DEFAULT[3], PAINT_DEFAULT[4]);
        gl.vertexAttrib2f(5, 1, 0);
        gl.drawArrays(gl.TRIANGLES, 0, m.checker.corners);
        gl.uniform1f(checker, 0);
        continue;
      }
      if (this.showGroups && m.expanded) {
        gl.uniform1f(painted, 1);
        gl.bindVertexArray(m.expanded.vao);
        gl.vertexAttrib2f(4, PAINT_DEFAULT[3], PAINT_DEFAULT[4]);
        gl.vertexAttrib2f(5, 1, 0);
        gl.drawArrays(gl.TRIANGLES, 0, m.expanded.corners);
        continue;
      }
      gl.uniform1f(painted, m.paintBuf ? 1 : 0);
      gl.bindVertexArray(m.vao);
      if (!m.paintBuf) {
        gl.vertexAttrib3f(3, PAINT_DEFAULT[0], PAINT_DEFAULT[1], PAINT_DEFAULT[2]);
        gl.vertexAttrib2f(4, PAINT_DEFAULT[3], PAINT_DEFAULT[4]);
        gl.vertexAttrib2f(5, 1, 0);
      }
      gl.drawElements(gl.TRIANGLES, m.count, gl.UNSIGNED_INT, 0);
    }
    gl.bindVertexArray(null);
    if (shadowMap && L.ground && this.shadow && this.groundBox) this.drawGround(vp, shadowMap);
    if (this.reference && this.referencePlace) this.drawReference(vp);
    if (post) {
      const proj = camera.projection();
      const inv = invert(proj);
      if (inv) post.finish(p, proj, inv, camera.distance * 0.1, camera.distance);
    }
  }

  private post: PostFx | null = null;
  private shadowRange = 1;
  private postSettings: PostSettings = { ...NO_POST };
  private shadow: { fb: WebGLFramebuffer; tex: WebGLTexture; size: number } | null = null;
  private shadowProgram: WebGLProgram | null = null;
  private reference: { tex: WebGLTexture; aspect: number } | null = null;
  private referencePlace: string | null = null;
  private referenceOpacity = 0.45;
  private referenceProgram: WebGLProgram | null = null;
  private referenceVao: WebGLVertexArrayObject | null = null;

  /** The image for the reference plane (null removes it). */
  setReferenceImage(image: TexImageSource | null): void {
    const gl = this.gl;
    if (this.reference) gl.deleteTexture(this.reference.tex);
    this.reference = null;
    if (!image) return;
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
    gl.generateMipmap(gl.TEXTURE_2D);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    const size = image as { width: number; height: number };
    this.reference = { tex, aspect: size.width / Math.max(1, size.height) };
  }

  /** Places the reference image on a plane behind the scene: front, side or top (null: not in the scene). */
  setReferencePlane(place: string | null, opacity: number): void {
    this.referencePlace = place;
    this.referenceOpacity = opacity;
  }

  /** The image plane: a textured quad just behind the scene's bounds, facing the matching view, sized to the scene. */
  private drawReference(vp: Float32Array): void {
    const gl = this.gl;
    const ref = this.reference;
    if (!ref) return;
    if (!this.referenceProgram) {
      this.referenceProgram = compile(gl, REFERENCE_VS, REFERENCE_FS);
      this.referenceVao = gl.createVertexArray();
    }
    const lo = [Infinity, Infinity, Infinity];
    const hi = [-Infinity, -Infinity, -Infinity];
    for (const m of this.meshes.values()) for (let c = 0; c < 3; c++) {
      lo[c] = Math.min(lo[c], m.lo[c]);
      hi[c] = Math.max(hi[c], m.hi[c]);
    }
    if (!Number.isFinite(lo[0])) lo.splice(0, 3, -1, -1, -1), hi.splice(0, 3, 1, 1, 1);
    const center = [(lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, (lo[2] + hi[2]) / 2];
    const half = Math.max(hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]) * 0.65;
    const gap = half * 0.15;
    // Origin, then the half-axes along the image's width and height.
    const [o, u, v] =
      this.referencePlace === 'side'
        ? [[lo[0] - gap, center[1], center[2]], [0, 0, -half * ref.aspect], [0, half, 0]]
        : this.referencePlace === 'top'
          ? [[center[0], lo[1] - gap, center[2]], [half * ref.aspect, 0, 0], [0, 0, -half]]
          : [[center[0], center[1], lo[2] - gap], [half * ref.aspect, 0, 0], [0, half, 0]];
    const prog = this.referenceProgram;
    gl.useProgram(prog);
    gl.uniformMatrix4fv(gl.getUniformLocation(prog, 'uViewProj'), false, vp);
    gl.uniform3fv(gl.getUniformLocation(prog, 'uOrigin'), o);
    gl.uniform3fv(gl.getUniformLocation(prog, 'uAxisU'), u);
    gl.uniform3fv(gl.getUniformLocation(prog, 'uAxisV'), v);
    gl.uniform1f(gl.getUniformLocation(prog, 'uOpacity'), this.referenceOpacity);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, ref.tex);
    gl.uniform1i(gl.getUniformLocation(prog, 'uImage'), 0);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    gl.depthMask(false);
    gl.bindVertexArray(this.referenceVao);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    gl.bindVertexArray(null);
    gl.depthMask(true);
    gl.disable(gl.BLEND);
  }

  private groundProgram: WebGLProgram | null = null;
  private groundVao: WebGLVertexArrayObject | null = null;
  private groundBox: { y: number; center: [number, number, number]; r: number } | null = null;

  /** The shadow catcher: a floor quad just under the scene, drawn as shadow only (blended, no colour of its own). */
  private drawGround(vp: Float32Array, shadowVP: Float32Array): void {
    const gl = this.gl;
    const g = this.groundBox;
    if (!g || !this.shadow) return;
    if (!this.groundProgram) {
      this.groundProgram = compile(gl, GROUND_VS, GROUND_FS);
      this.groundVao = gl.createVertexArray();
    }
    const prog = this.groundProgram;
    gl.useProgram(prog);
    gl.uniformMatrix4fv(gl.getUniformLocation(prog, 'uViewProj'), false, vp);
    gl.uniformMatrix4fv(gl.getUniformLocation(prog, 'uShadowVP'), false, shadowVP);
    gl.uniform3f(gl.getUniformLocation(prog, 'uCenter'), g.center[0], g.y - g.r * 0.002, g.center[2]);
    gl.uniform1f(gl.getUniformLocation(prog, 'uHalf'), g.r * 0.9);
    gl.activeTexture(gl.TEXTURE2);
    gl.bindTexture(gl.TEXTURE_2D, this.shadow.tex);
    gl.uniform1i(gl.getUniformLocation(prog, 'uShadowMap'), 2);
    gl.activeTexture(gl.TEXTURE0);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    gl.depthMask(false);
    gl.bindVertexArray(this.groundVao);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    gl.bindVertexArray(null);
    gl.depthMask(true);
    gl.disable(gl.BLEND);
  }

  /** Draws every mesh's depth from the key light into a 2048 square map; returns the light's view-projection. */
  private renderShadow(keyWorld: [number, number, number]): Float32Array | null {
    const gl = this.gl;
    const size = 2048;
    if (!this.shadow) {
      const tex = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.DEPTH_COMPONENT24, size, size, 0, gl.DEPTH_COMPONENT, gl.UNSIGNED_INT, null);
      for (const p of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER]) gl.texParameteri(gl.TEXTURE_2D, p, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      const fb = gl.createFramebuffer();
      gl.bindFramebuffer(gl.FRAMEBUFFER, fb);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.TEXTURE_2D, tex, 0);
      this.shadow = { fb, tex, size };
      this.shadowProgram = compile(gl, SHADOW_VS, SHADOW_FS);
    }
    // Fit an orthographic light camera around every mesh's bounds.
    const lo = [Infinity, Infinity, Infinity];
    const hi = [-Infinity, -Infinity, -Infinity];
    for (const m of this.meshes.values()) for (let c = 0; c < 3; c++) {
      lo[c] = Math.min(lo[c], m.lo[c]);
      hi[c] = Math.max(hi[c], m.hi[c]);
    }
    const center: [number, number, number] = [(lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, (lo[2] + hi[2]) / 2];
    // The radius covers the ground shadow's floor too (twice the scene's footprint), so its shadow is not cut off.
    const r = Math.max(1e-3, Math.hypot(hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]) * 1.1);
    this.groundBox = { y: lo[1], center, r };
    const eye: [number, number, number] = [center[0] + keyWorld[0] * r * 2, center[1] + keyWorld[1] * r * 2, center[2] + keyWorld[2] * r * 2];
    const up: [number, number, number] = Math.abs(keyWorld[1]) > 0.99 ? [0, 0, 1] : [0, 1, 0];
    const vp = multiply(ortho(r, 1, r * 0.5, r * 3.5), lookAt(eye, center, up));
    // The orthographic depth is linear over 3r, which turns a depth difference into a thickness in world units.
    this.shadowRange = r * 3;
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.shadow.fb);
    gl.viewport(0, 0, size, size);
    gl.clear(gl.DEPTH_BUFFER_BIT);
    const prog = this.shadowProgram as WebGLProgram;
    gl.useProgram(prog);
    gl.uniformMatrix4fv(gl.getUniformLocation(prog, 'uLightVP'), false, vp);
    const model = gl.getUniformLocation(prog, 'uModel');
    for (const [id, m] of this.meshes) {
      gl.uniformMatrix4fv(model, false, this.previews.get(id) ?? IDENTITY);
      gl.bindVertexArray(m.vao);
      gl.drawElements(gl.TRIANGLES, m.count, gl.UNSIGNED_INT, 0);
    }
    gl.bindVertexArray(null);
    return vp;
  }

  /** Turns post effects on or off; the offscreen pipeline is built on first use. */
  setPost(settings: PostSettings): void {
    this.postSettings = { ...settings };
    if (!this.post && (settings.ssao || settings.bloom || settings.dof || settings.vignette || settings.grain)) this.post = new PostFx(this.gl);
  }

  private env: { tex: WebGLTexture; levels: number } | null = null;
  lighting: Lighting = { environment: false, background: false, rotation: 0, exposure: 1, lights: 0.35, tone: 'reinhard', blur: 0.35, keyAzimuth: -36.9, keyElevation: 40.9, keyIntensity: 1, keyColor: [1, 0.94, 0.87], keyFollowsView: true, shadows: false, ground: false, extra: [] };

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

const SHADOW_VS = `#version 300 es
layout(location=0) in vec3 aPos;
uniform mat4 uLightVP;
uniform mat4 uModel;
void main() { gl_Position = uLightVP * uModel * vec4(aPos, 1.0); }`;

const REFERENCE_VS = `#version 300 es
uniform mat4 uViewProj;
uniform vec3 uOrigin;
uniform vec3 uAxisU;
uniform vec3 uAxisV;
out vec2 vUv;
void main() {
  vec2 c = vec2(float(gl_VertexID & 1), float((gl_VertexID >> 1) & 1));
  vUv = vec2(c.x, 1.0 - c.y);
  gl_Position = uViewProj * vec4(uOrigin + uAxisU * (c.x * 2.0 - 1.0) + uAxisV * (c.y * 2.0 - 1.0), 1.0);
}`;

const REFERENCE_FS = `#version 300 es
precision mediump float;
in vec2 vUv;
uniform sampler2D uImage;
uniform float uOpacity;
out vec4 outColor;
void main() {
  vec4 c = texture(uImage, vUv);
  outColor = vec4(c.rgb, c.a * uOpacity);
}`;

const GROUND_VS = `#version 300 es
uniform mat4 uViewProj;
uniform vec3 uCenter;
uniform float uHalf;
out vec3 vWorld;
out vec2 vLocal;
void main() {
  vec2 c = vec2(float(gl_VertexID & 1), float((gl_VertexID >> 1) & 1)) * 2.0 - 1.0;
  vLocal = c;
  vWorld = uCenter + vec3(c.x, 0.0, c.y) * uHalf;
  gl_Position = uViewProj * vec4(vWorld, 1.0);
}`;

const GROUND_FS = `#version 300 es
precision highp float;
in vec3 vWorld;
in vec2 vLocal;
uniform mat4 uShadowVP;
uniform sampler2D uShadowMap;
out vec4 outColor;
void main() {
  vec4 c = uShadowVP * vec4(vWorld, 1.0);
  vec3 q = c.xyz / c.w * 0.5 + 0.5;
  vec2 texel = 1.0 / vec2(textureSize(uShadowMap, 0));
  float shade = 0.0;
  for (int x = -2; x <= 2; x++)
    for (int y = -2; y <= 2; y++) shade += q.z - 0.004 > texture(uShadowMap, q.xy + vec2(x, y) * texel * 1.5).r ? 1.0 : 0.0;
  float fade = 1.0 - smoothstep(0.6, 1.0, length(vLocal));
  outColor = vec4(0.0, 0.0, 0.0, shade / 25.0 * 0.55 * fade);
}`;

const SHADOW_FS = `#version 300 es
precision mediump float;
out vec4 outColor;
void main() { outColor = vec4(1.0); }`;

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
  /** The key light, in degrees: azimuth from straight ahead (or world +z), elevation above the horizon. */
  keyAzimuth: number;
  keyElevation: number;
  keyIntensity: number;
  /** Linear 0..1 colour. */
  keyColor: [number, number, number];
  /** The key light turns with the view (a headlight) or stays put in the world. */
  keyFollowsView: boolean;
  /** The key light casts shadows. */
  shadows: boolean;
  /** A shadow catcher: an invisible floor under the scene that shows only the shadow cast on it. */
  ground: boolean;
  /** Up to three added lights. */
  extra: LightSpec[];
}

/** An object's light transport in lit shading (the engine's set_material). */
export interface ObjectMaterial {
  subsurface: number;
  subsurfaceColor: [number, number, number];
  subsurfaceRadius: number;
  transmission: number;
  ior: number;
}

export interface LightSpec {
  azimuth: number;
  elevation: number;
  intensity: number;
  color: [number, number, number];
  followsView: boolean;
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

const GRADIENT_FS = `#version 300 es
precision highp float;
in vec2 vNdc;
uniform vec3 uTop;
uniform vec3 uBottom;
uniform float uMid;
uniform float uVignette;
uniform vec2 uSize;
out vec4 outColor;
float hash(vec2 p) { return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453); }
void main() {
  vec2 uv = vNdc * 0.5 + 0.5;
  float m = clamp(uMid, 0.05, 0.95);
  float t = uv.y < m ? 0.5 * uv.y / m : 0.5 + 0.5 * (uv.y - m) / (1.0 - m);
  vec3 c = mix(uBottom, uTop, smoothstep(0.0, 1.0, t));
  vec2 d = (uv - vec2(0.5, 0.55)) * vec2(uSize.x / max(uSize.y, 1.0), 1.0);
  c *= 1.0 - uVignette * smoothstep(0.25, 1.15, length(d));
  c += (hash(gl_FragCoord.xy) - 0.5) / 255.0;
  outColor = vec4(c, 1.0);
}`;

const FLOOR_FS = `#version 300 es
precision highp float;
in vec2 vNdc;
uniform mat4 uInvViewProj;
uniform vec3 uEye;
uniform vec3 uViewDir;
uniform float uOrtho;
uniform float uScale;
uniform float uFrac;
uniform float uFade;
uniform vec3 uLine;
uniform float uAlpha;
out vec4 outColor;
vec3 unproject(float z) { vec4 p = uInvViewProj * vec4(vNdc, z, 1.0); return p.xyz / p.w; }
float lines(vec2 p, float step) {
  vec2 c = p / step;
  vec2 w = max(fwidth(c), vec2(1e-5));
  vec2 g = abs(fract(c - 0.5) - 0.5) / w;
  return 1.0 - min(min(g.x, g.y), 1.0);
}
void main() {
  vec3 a = unproject(-1.0);
  vec3 b = unproject(1.0);
  vec3 dir = b - a;
  if (abs(dir.y) < 1e-7) discard;
  float t = -a.y / dir.y;
  if (t < 0.0 || t > 1.0) discard;
  vec3 p = a + dir * t;
  vec3 toEye = uOrtho > 0.5 ? -uViewDir : normalize(uEye - p);
  float grazing = smoothstep(0.04, 0.3, abs(toEye.y));
  float fade = 1.0 - smoothstep(uFade * 0.3, uFade, length(p.xz - uEye.xz));
  float minor = lines(p.xz, uScale * 0.1) * (1.0 - uFrac) * 0.45;
  float major = lines(p.xz, uScale);
  float big = lines(p.xz, uScale * 10.0) * uFrac;
  vec2 axis = 1.0 - min(abs(p.xz) / max(fwidth(p.xz) * 1.5, vec2(1e-5)), vec2(1.0));
  float grid = max(minor, max(major * 0.75, big * 0.75));
  vec3 color = uLine;
  float alpha = grid * uAlpha;
  if (axis.y > 0.0) { color = mix(color, vec3(0.85, 0.3, 0.32), axis.y); alpha = max(alpha, axis.y * 0.9); }
  if (axis.x > 0.0) { color = mix(color, vec3(0.3, 0.48, 0.9), axis.x); alpha = max(alpha, axis.x * 0.9); }
  alpha *= fade * grazing;
  if (alpha < 0.004) discard;
  outColor = vec4(color, alpha);
}`;

/** The viewport's backdrop: a vertical studio gradient (with a vignette) or a flat colour, and the floor grid. */
export interface Backdrop {
  mode: 'gradient' | 'color';
  top: [number, number, number];
  bottom: [number, number, number];
  mid: number;
  vignette: number;
  grid: boolean;
  gridColor: [number, number, number];
}

export function hexColor(css: string): [number, number, number] | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(css.trim());
  if (!m) return null;
  const v = parseInt(m[1], 16);
  return [((v >> 16) & 255) / 255, ((v >> 8) & 255) / 255, (v & 255) / 255];
}
