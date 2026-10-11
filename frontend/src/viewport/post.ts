/**
 * Post-processing for the viewport: the scene is drawn into an offscreen colour and depth target, then screen-space
 * ambient occlusion (from depth alone), bloom (a bright pass blurred at quarter size), depth of field (focused on the
 * orbit target), a vignette and film grain are applied in a final pass to the canvas. Each effect is off by default and
 * costs nothing while off; with every effect off the renderer draws straight to the canvas as before.
 */

export interface PostSettings {
  ssao: boolean;
  bloom: boolean;
  dof: boolean;
  vignette: boolean;
  grain: boolean;
}

export const NO_POST: PostSettings = { ssao: false, bloom: false, dof: false, vignette: false, grain: false };

const FULLSCREEN_VS = `#version 300 es
out vec2 vUv;
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  vUv = p;
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}`;

const SSAO_FS = `#version 300 es
precision highp float;
in vec2 vUv;
uniform sampler2D uDepth;
uniform mat4 uProj;
uniform mat4 uInvProj;
uniform float uRadius;
out vec4 outColor;
vec3 viewPos(vec2 uv) {
  float d = texture(uDepth, uv).r;
  vec4 p = uInvProj * vec4(uv * 2.0 - 1.0, d * 2.0 - 1.0, 1.0);
  return p.xyz / p.w;
}
float hash(vec2 p) { return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453); }
void main() {
  float d = texture(uDepth, vUv).r;
  if (d >= 1.0) { outColor = vec4(1.0); return; }
  vec3 p = viewPos(vUv);
  vec3 n = normalize(cross(dFdx(p), dFdy(p)));
  if (dot(n, p) > 0.0) n = -n;
  // The kernel turns by a 4 by 4 interleaved pattern, which the composite's 4 by 4 blur cancels exactly, instead of
  // per-pixel noise, which leaves a mottled surface.
  vec2 cell = mod(floor(gl_FragCoord.xy), 4.0);
  float a = (cell.x + cell.y * 4.0 + 0.5) / 16.0 * 6.2831853;
  vec3 r = normalize(vec3(cos(a), sin(a), 0.0));
  vec3 t = normalize(r - n * dot(r, n));
  vec3 b = cross(n, t);
  float occlusion = 0.0;
  const int N = 16;
  for (int i = 0; i < N; i++) {
    float fi = float(i);
    float h1 = hash(vec2(fi * 1.7, 0.31));
    float h2 = hash(vec2(fi * 3.1, 0.57));
    float h3 = hash(vec2(fi * 5.3, 0.83));
    vec3 dir = normalize(vec3(h1 * 2.0 - 1.0, h2 * 2.0 - 1.0, h3 * 0.8 + 0.2));
    float s = (fi + 1.0) / float(N);
    s = mix(0.1, 1.0, s * s);
    vec3 q = p + (t * dir.x + b * dir.y + n * dir.z) * uRadius * s;
    vec4 c = uProj * vec4(q, 1.0);
    vec2 uv = c.xy / c.w * 0.5 + 0.5;
    if (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0) continue;
    float sceneZ = viewPos(uv).z;
    float range = smoothstep(0.0, 1.0, uRadius / max(abs(p.z - sceneZ), 1e-5));
    // The bias keeps a smooth surface from shadowing itself through depth-buffer precision.
    occlusion += (sceneZ >= q.z + uRadius * 0.08 ? 1.0 : 0.0) * range;
  }
  float open = 1.0 - occlusion / float(N);
  outColor = vec4(vec3(open * open), 1.0);
}`;

const BRIGHT_FS = `#version 300 es
precision highp float;
in vec2 vUv;
uniform sampler2D uColor;
out vec4 outColor;
void main() {
  vec3 c = texture(uColor, vUv).rgb;
  float l = dot(c, vec3(0.2126, 0.7152, 0.0722));
  outColor = vec4(c * smoothstep(0.62, 0.95, l), 1.0);
}`;

const BLUR_FS = `#version 300 es
precision highp float;
in vec2 vUv;
uniform sampler2D uSource;
uniform vec2 uStep;
out vec4 outColor;
void main() {
  const float w[5] = float[5](0.227, 0.195, 0.122, 0.054, 0.016);
  vec3 c = texture(uSource, vUv).rgb * w[0];
  for (int i = 1; i < 5; i++) {
    c += texture(uSource, vUv + uStep * float(i)).rgb * w[i];
    c += texture(uSource, vUv - uStep * float(i)).rgb * w[i];
  }
  outColor = vec4(c, 1.0);
}`;

const COMPOSITE_FS = `#version 300 es
precision highp float;
in vec2 vUv;
uniform sampler2D uColor;
uniform sampler2D uDepth;
uniform sampler2D uAo;
uniform sampler2D uBloom;
uniform mat4 uInvProj;
uniform vec2 uTexel;
uniform float uSsao;
uniform float uBloomOn;
uniform float uDof;
uniform float uFocus;
uniform float uVignette;
uniform float uGrain;
uniform float uTime;
out vec4 outColor;
float viewDepth(vec2 uv) {
  float d = texture(uDepth, uv).r;
  vec4 p = uInvProj * vec4(uv * 2.0 - 1.0, d * 2.0 - 1.0, 1.0);
  return -p.z / p.w;
}
float hash(vec2 p) { return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453); }
void main() {
  vec3 c = texture(uColor, vUv).rgb;
  if (uDof > 0.5) {
    // Circle of confusion from the distance to the focus plane, up to 9 pixels; a golden-angle disc of samples
    // that ignores nearer, sharper samples so edges do not bleed onto the background.
    float z = viewDepth(vUv);
    float coc = clamp(abs(z - uFocus) / max(uFocus, 1e-4) * 14.0, 0.0, 9.0);
    if (coc > 0.5) {
      vec3 sum = c;
      float total = 1.0;
      for (int i = 1; i < 24; i++) {
        float fi = float(i);
        float r = sqrt(fi / 24.0) * coc;
        float a = fi * 2.39996;
        vec2 uv = vUv + vec2(cos(a), sin(a)) * r * uTexel;
        float zs = viewDepth(uv);
        float w = zs + 1e-3 < z && abs(zs - uFocus) < abs(z - uFocus) ? 0.3 : 1.0;
        sum += texture(uColor, uv).rgb * w;
        total += w;
      }
      c = sum / total;
    }
  }
  if (uSsao > 0.5) {
    float ao = 0.0;
    for (int x = -2; x <= 1; x++)
      for (int y = -2; y <= 1; y++) ao += texture(uAo, vUv + (vec2(x, y) + 0.5) * uTexel * 2.0).r;
    c *= mix(1.0, ao / 16.0, 0.85);
  }
  if (uBloomOn > 0.5) c += texture(uBloom, vUv).rgb * 0.9;
  if (uVignette > 0.5) {
    vec2 q = vUv - 0.5;
    c *= 1.0 - smoothstep(0.35, 0.85, length(q) * 1.15) * 0.55;
  }
  if (uGrain > 0.5) c += (hash(vUv * 1013.0 + uTime) - 0.5) * 0.045;
  outColor = vec4(clamp(c, 0.0, 1.0), 1.0);
}`;

interface Target {
  fb: WebGLFramebuffer;
  tex: WebGLTexture;
  w: number;
  h: number;
}

function compile(gl: WebGL2RenderingContext, fs: string): WebGLProgram {
  const program = gl.createProgram();
  for (const [type, src] of [[gl.VERTEX_SHADER, FULLSCREEN_VS], [gl.FRAGMENT_SHADER, fs]] as const) {
    const shader = gl.createShader(type);
    if (!shader) throw new Error('shader allocation failed');
    gl.shaderSource(shader, src);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader) ?? 'post shader compile failed');
    gl.attachShader(program, shader);
  }
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program) ?? 'post link failed');
  return program;
}

function texture(gl: WebGL2RenderingContext, w: number, h: number): WebGLTexture {
  const tex = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, tex);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  return tex;
}

export class PostFx {
  private ssao: WebGLProgram;
  private bright: WebGLProgram;
  private blur: WebGLProgram;
  private composite: WebGLProgram;
  private vao: WebGLVertexArrayObject;
  private scene: { fb: WebGLFramebuffer; color: WebGLTexture; depth: WebGLTexture; msaa: WebGLFramebuffer; rbColor: WebGLRenderbuffer; rbDepth: WebGLRenderbuffer; resolveDepth: WebGLFramebuffer; w: number; h: number } | null = null;
  private ao: Target | null = null;
  private bloomA: Target | null = null;
  private bloomB: Target | null = null;

  constructor(private gl: WebGL2RenderingContext) {
    this.ssao = compile(gl, SSAO_FS);
    this.bright = compile(gl, BRIGHT_FS);
    this.blur = compile(gl, BLUR_FS);
    this.composite = compile(gl, COMPOSITE_FS);
    this.vao = gl.createVertexArray();
  }

  private target(old: Target | null, w: number, h: number): Target {
    const gl = this.gl;
    if (old && old.w === w && old.h === h) return old;
    if (old) {
      gl.deleteFramebuffer(old.fb);
      gl.deleteTexture(old.tex);
    }
    const tex = texture(gl, w, h);
    const fb = gl.createFramebuffer();
    gl.bindFramebuffer(gl.FRAMEBUFFER, fb);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
    return { fb, tex, w, h };
  }

  /** Binds the offscreen target the scene is drawn into (multisampled, resolved in finish). */
  begin(w: number, h: number): void {
    const gl = this.gl;
    let s = this.scene;
    if (!s || s.w !== w || s.h !== h) {
      if (s) {
        for (const f of [s.fb, s.msaa, s.resolveDepth]) gl.deleteFramebuffer(f);
        gl.deleteTexture(s.color);
        gl.deleteTexture(s.depth);
        gl.deleteRenderbuffer(s.rbColor);
        gl.deleteRenderbuffer(s.rbDepth);
      }
      const samples = Math.min(4, gl.getParameter(gl.MAX_SAMPLES) as number);
      const msaa = gl.createFramebuffer();
      gl.bindFramebuffer(gl.FRAMEBUFFER, msaa);
      const rbColor = gl.createRenderbuffer();
      gl.bindRenderbuffer(gl.RENDERBUFFER, rbColor);
      gl.renderbufferStorageMultisample(gl.RENDERBUFFER, samples, gl.RGBA8, w, h);
      gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.RENDERBUFFER, rbColor);
      const rbDepth = gl.createRenderbuffer();
      gl.bindRenderbuffer(gl.RENDERBUFFER, rbDepth);
      gl.renderbufferStorageMultisample(gl.RENDERBUFFER, samples, gl.DEPTH_COMPONENT24, w, h);
      gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.RENDERBUFFER, rbDepth);
      const color = texture(gl, w, h);
      const fb = gl.createFramebuffer();
      gl.bindFramebuffer(gl.FRAMEBUFFER, fb);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, color, 0);
      const depth = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, depth);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.DEPTH_COMPONENT24, w, h, 0, gl.DEPTH_COMPONENT, gl.UNSIGNED_INT, null);
      for (const p of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER]) gl.texParameteri(gl.TEXTURE_2D, p, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      // Depth cannot be resolved from a multisampled buffer into a texture with colour in the same blit on every
      // driver, so it gets its own framebuffer.
      const resolveDepth = gl.createFramebuffer();
      gl.bindFramebuffer(gl.FRAMEBUFFER, resolveDepth);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.TEXTURE_2D, depth, 0);
      s = { fb, color, depth, msaa, rbColor, rbDepth, resolveDepth, w, h };
      this.scene = s;
    }
    gl.bindFramebuffer(gl.FRAMEBUFFER, s.msaa);
  }

  private pass(program: WebGLProgram, target: Target | null, w: number, h: number): void {
    const gl = this.gl;
    gl.bindFramebuffer(gl.FRAMEBUFFER, target ? target.fb : null);
    gl.viewport(0, 0, w, h);
    gl.useProgram(program);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  private bind(program: WebGLProgram, name: string, unit: number, tex: WebGLTexture): void {
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0 + unit);
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.uniform1i(gl.getUniformLocation(program, name), unit);
  }

  /** Resolves the scene and draws it to the canvas with the chosen effects. */
  finish(settings: PostSettings, proj: Float32Array, invProj: Float32Array, radius: number, focus: number): void {
    const gl = this.gl;
    const s = this.scene;
    if (!s) return;
    const { w, h } = s;
    gl.bindFramebuffer(gl.READ_FRAMEBUFFER, s.msaa);
    gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, s.fb);
    gl.blitFramebuffer(0, 0, w, h, 0, 0, w, h, gl.COLOR_BUFFER_BIT, gl.NEAREST);
    gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, s.resolveDepth);
    gl.blitFramebuffer(0, 0, w, h, 0, 0, w, h, gl.DEPTH_BUFFER_BIT, gl.NEAREST);
    gl.disable(gl.DEPTH_TEST);
    gl.bindVertexArray(this.vao);
    if (settings.ssao) {
      // At half size: occlusion is soft, and the composite blurs it while upsampling.
      this.ao = this.target(this.ao, Math.max(1, w >> 1), Math.max(1, h >> 1));
      gl.useProgram(this.ssao);
      this.bind(this.ssao, 'uDepth', 0, s.depth);
      gl.uniformMatrix4fv(gl.getUniformLocation(this.ssao, 'uProj'), false, proj);
      gl.uniformMatrix4fv(gl.getUniformLocation(this.ssao, 'uInvProj'), false, invProj);
      gl.uniform1f(gl.getUniformLocation(this.ssao, 'uRadius'), radius);
      this.pass(this.ssao, this.ao, this.ao.w, this.ao.h);
    }
    if (settings.bloom) {
      const bw = Math.max(1, w >> 2);
      const bh = Math.max(1, h >> 2);
      this.bloomA = this.target(this.bloomA, bw, bh);
      this.bloomB = this.target(this.bloomB, bw, bh);
      gl.useProgram(this.bright);
      this.bind(this.bright, 'uColor', 0, s.color);
      this.pass(this.bright, this.bloomA, bw, bh);
      for (let k = 0; k < 2; k++) {
        gl.useProgram(this.blur);
        this.bind(this.blur, 'uSource', 0, this.bloomA.tex);
        gl.uniform2f(gl.getUniformLocation(this.blur, 'uStep'), (1.5 + k) / bw, 0);
        this.pass(this.blur, this.bloomB, bw, bh);
        this.bind(this.blur, 'uSource', 0, this.bloomB.tex);
        gl.uniform2f(gl.getUniformLocation(this.blur, 'uStep'), 0, (1.5 + k) / bh);
        this.pass(this.blur, this.bloomA, bw, bh);
      }
    }
    const c = this.composite;
    gl.useProgram(c);
    this.bind(c, 'uColor', 0, s.color);
    this.bind(c, 'uDepth', 1, s.depth);
    this.bind(c, 'uAo', 2, this.ao?.tex ?? s.color);
    this.bind(c, 'uBloom', 3, this.bloomA?.tex ?? s.color);
    gl.uniformMatrix4fv(gl.getUniformLocation(c, 'uInvProj'), false, invProj);
    gl.uniform2f(gl.getUniformLocation(c, 'uTexel'), 1 / w, 1 / h);
    gl.uniform1f(gl.getUniformLocation(c, 'uSsao'), settings.ssao ? 1 : 0);
    gl.uniform1f(gl.getUniformLocation(c, 'uBloomOn'), settings.bloom ? 1 : 0);
    gl.uniform1f(gl.getUniformLocation(c, 'uDof'), settings.dof ? 1 : 0);
    gl.uniform1f(gl.getUniformLocation(c, 'uFocus'), focus);
    gl.uniform1f(gl.getUniformLocation(c, 'uVignette'), settings.vignette ? 1 : 0);
    gl.uniform1f(gl.getUniformLocation(c, 'uGrain'), settings.grain ? 1 : 0);
    gl.uniform1f(gl.getUniformLocation(c, 'uTime'), (performance.now() % 1000) / 1000);
    this.pass(c, null, w, h);
    gl.bindVertexArray(null);
    gl.activeTexture(gl.TEXTURE0);
    gl.enable(gl.DEPTH_TEST);
  }
}
