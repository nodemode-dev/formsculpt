// The browser engine outside the browser: loads frontend/public/engine/fsc_engine.wasm in Node with the same imports
// the page gives it and drives its C ABI the way frontend/src/engine/wasm.ts does. Run by ./formsculpt test engine
// when the module has been built (./formsculpt engine wasm).
import { readFileSync } from 'node:fs';

const path = process.argv[2];
let memory;
let panic = '';
const { instance } = await WebAssembly.instantiate(readFileSync(path), {
  env: {
    fsc_now_ms: () => performance.now(),
    fsc_log: (ptr, len) => {
      panic = new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, len));
    },
  },
});
const x = instance.exports;
memory = x.memory;
x.fsc_init();
const enc = new TextEncoder();
const dec = new TextDecoder();
const held = (bytes) => {
  const ptr = x.fsc_alloc(bytes.length);
  new Uint8Array(memory.buffer, ptr, bytes.length).set(bytes);
  return [ptr, bytes.length];
};
const out = () => new Uint8Array(memory.buffer, x.fsc_out_ptr(), x.fsc_out_len()).slice();
const call = (name, args = {}) => {
  const [np, nl] = held(enc.encode(name));
  const [ap, al] = held(enc.encode(JSON.stringify(args)));
  x.fsc_dispatch(np, nl, ap, al);
  x.fsc_free(np, nl);
  x.fsc_free(ap, al);
  return JSON.parse(dec.decode(out()));
};
let failures = 0;
const check = (label, ok) => {
  console.log(`${ok ? 'ok  ' : 'FAIL'}  ${label}`);
  if (!ok) failures += 1;
};

try {
  x.fsc_catalogue();
  const catalogue = JSON.parse(dec.decode(out()));
  check(`the catalogue lists ${catalogue.commands.length} commands`, catalogue.commands.length > 90);
  const added = call('add_primitive', { kind: 'sphere', detail: 4 });
  const id = added.result.object.id;
  check('a sphere is added', added.ok);
  check('its mesh payload is FSCM', x.fsc_mesh(id, 0) === 0 && dec.decode(out().slice(0, 4)) === 'FSCM');
  const [sp, sl] = held(enc.encode(JSON.stringify({ brush: 'draw', radius: 0.3, strength: 1 })));
  check('a live stroke begins', x.fsc_stroke_begin(id, sp, sl) === 0);
  x.fsc_free(sp, sl);
  const flags = x.fsc_stroke_sample(0, 0, 5, 0, 0, -1, 1, 0);
  check('a sample hits and sends an FSCU update', (flags & 1) === 1 && dec.decode(out().slice(0, 4)) === 'FSCU');
  x.fsc_stroke_end();
  const ended = JSON.parse(dec.decode(out()));
  check('the stroke ends as one step', ended.ok && ended.result.vertices_changed > 0);
  check('undo works', call('undo').ok);
  check('a voxel remesh runs without threads', call('remesh_voxel', { resolution: 48 }).ok);
  const view = call('get_view', { width: 96, height: 96 });
  check('get_view renders (the page lends the clock)', view.ok && view.result.coverage > 0.05);
  check('a project saves', x.fsc_save() === 0 && out().length > 1000);
  const [en, el] = held(enc.encode('scene.usda'));
  check('a USD file exports', x.fsc_export(en, el, 0, 0) === 0 && dec.decode(out().slice(0, 9)) === '#usda 1.0');
  const cutter = call('add_primitive', { kind: 'cube', size: 0.8 }).result.object.id;
  call('transform_object', { id: cutter, translate: [0.5, 0.5, 0] });
  check('a live boolean preview is built exactly', x.fsc_boolean_preview(id, cutter, 1) === 0 && dec.decode(out().slice(0, 4)) === 'FSCM');
  check('an exact boolean commits', call('boolean', { a: id, b: cutter, op: 'difference' }).result?.method === 'exact');
  check('an unknown command is an error, not a trap', call('no_such').error.code === 'unknown_command');
} catch (err) {
  console.log(`FAIL  the module trapped: ${panic || err}`);
  failures += 1;
}
if (failures) {
  console.log(`${failures} failed`);
  process.exit(1);
}
console.log('browser engine passed');
