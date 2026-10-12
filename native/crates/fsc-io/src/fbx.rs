//! Binary FBX: a writer for version 7.4 (meshes with normals, vertex colours and sculpt layers as blend shapes) and a
//! reader for 7.x binary files (meshes, their model transforms and vertex colours, arrays plain or zlib-compressed).
//! Written from the published description of the node-record layout; no FBX SDK code is used.

use glam::{EulerRot, Mat4, Quat, Vec3};

use fsc_core::Mesh;

use crate::formats::Named;
use crate::gltf::GltfObject;

const MAGIC: &[u8] = b"Kaydara FBX Binary  \x00\x1a\x00";
const VERSION: u32 = 7400;

// ------------------------------------------------------------------ writing

enum P {
    I32(i32),
    I64(i64),
    F64(f64),
    Str(Vec<u8>),
    F64s(Vec<f64>),
    I32s(Vec<i32>),
}

struct Node {
    name: &'static str,
    props: Vec<P>,
    children: Vec<Node>,
}

fn node(name: &'static str, props: Vec<P>) -> Node {
    Node { name, props, children: Vec::new() }
}

fn parent(name: &'static str, props: Vec<P>, children: Vec<Node>) -> Node {
    Node { name, props, children }
}

fn s(text: &str) -> P {
    P::Str(text.as_bytes().to_vec())
}

/// An object's name and class as FBX joins them: Name\0\x01Class.
fn named(name: &str, class: &str) -> P {
    let mut b = name.as_bytes().to_vec();
    b.extend_from_slice(b"\x00\x01");
    b.extend_from_slice(class.as_bytes());
    P::Str(b)
}

fn prop70(name: &str, kind: &str, label: &str, flags: &str, values: Vec<P>) -> Node {
    let mut props = vec![s(name), s(kind), s(label), s(flags)];
    props.extend(values);
    node("P", props)
}

fn write_props(out: &mut Vec<u8>, props: &[P]) {
    for p in props {
        match p {
            P::I32(v) => {
                out.push(b'I');
                out.extend_from_slice(&v.to_le_bytes());
            }
            P::I64(v) => {
                out.push(b'L');
                out.extend_from_slice(&v.to_le_bytes());
            }
            P::F64(v) => {
                out.push(b'D');
                out.extend_from_slice(&v.to_le_bytes());
            }
            P::Str(b) => {
                out.push(b'S');
                out.extend_from_slice(&(b.len() as u32).to_le_bytes());
                out.extend_from_slice(b);
            }
            P::F64s(v) => {
                out.push(b'd');
                out.extend_from_slice(&(v.len() as u32).to_le_bytes());
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(&((v.len() * 8) as u32).to_le_bytes());
                for x in v {
                    out.extend_from_slice(&x.to_le_bytes());
                }
            }
            P::I32s(v) => {
                out.push(b'i');
                out.extend_from_slice(&(v.len() as u32).to_le_bytes());
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(&((v.len() * 4) as u32).to_le_bytes());
                for x in v {
                    out.extend_from_slice(&x.to_le_bytes());
                }
            }
        }
    }
}

/// One node record (7.4 layout: 32-bit end offset, property count and property bytes) with its children.
fn write_node(out: &mut Vec<u8>, n: &Node) {
    let start = out.len();
    out.extend_from_slice(&[0u8; 12]);
    out.push(n.name.len() as u8);
    out.extend_from_slice(n.name.as_bytes());
    let props_start = out.len();
    write_props(out, &n.props);
    let props_len = out.len() - props_start;
    if !n.children.is_empty() {
        for c in &n.children {
            write_node(out, c);
        }
        out.extend_from_slice(&[0u8; 13]);
    }
    let end = out.len() as u32;
    out[start..start + 4].copy_from_slice(&end.to_le_bytes());
    out[start + 4..start + 8].copy_from_slice(&(n.props.len() as u32).to_le_bytes());
    out[start + 8..start + 12].copy_from_slice(&(props_len as u32).to_le_bytes());
}

fn geometry_children(mesh: &Mesh) -> Vec<Node> {
    let vertices: Vec<f64> = mesh.positions.iter().flat_map(|p| p.iter().map(|&c| c as f64)).collect();
    // A polygon's last index is stored as -(index + 1).
    let polygons: Vec<i32> = mesh.indices.chunks_exact(3).flat_map(|t| [t[0] as i32, t[1] as i32, -(t[2] as i32) - 1]).collect();
    let normals: Vec<f64> = mesh.indices.iter().flat_map(|&v| mesh.normals[v as usize].iter().map(|&c| c as f64)).collect();
    let mut children = vec![
        node("GeometryVersion", vec![P::I32(124)]),
        node("Vertices", vec![P::F64s(vertices)]),
        node("PolygonVertexIndex", vec![P::I32s(polygons)]),
        parent(
            "LayerElementNormal",
            vec![P::I32(0)],
            vec![node("Version", vec![P::I32(101)]), node("Name", vec![s("")]), node("MappingInformationType", vec![s("ByPolygonVertex")]), node("ReferenceInformationType", vec![s("Direct")]), node("Normals", vec![P::F64s(normals)])],
        ),
    ];
    let mut layer = vec![node("Version", vec![P::I32(100)]), parent("LayerElement", vec![], vec![node("Type", vec![s("LayerElementNormal")]), node("TypedIndex", vec![P::I32(0)])])];
    if mesh.has_paint() {
        let colors: Vec<f64> = mesh.paint.iter().flat_map(|p| [p[0] as f64, p[1] as f64, p[2] as f64, 1.0]).collect();
        children.push(parent(
            "LayerElementColor",
            vec![P::I32(0)],
            vec![node("Version", vec![P::I32(101)]), node("Name", vec![s("Paint")]), node("MappingInformationType", vec![s("ByVertice")]), node("ReferenceInformationType", vec![s("Direct")]), node("Colors", vec![P::F64s(colors)])],
        ));
        layer.push(parent("LayerElement", vec![], vec![node("Type", vec![s("LayerElementColor")]), node("TypedIndex", vec![P::I32(0)])]));
    }
    children.push(parent("Layer", vec![P::I32(0)], layer));
    children
}

/// A binary FBX 7.4 file: one model and mesh per object (y up, units as metres-to-centimetres 1:1 like glTF scenes
/// read in other apps), vertex colours from paint, and each sculpt layer as a blend shape channel at its strength.
pub fn write_fbx(objects: &[GltfObject]) -> Vec<u8> {
    let mut objects_nodes = Vec::new();
    let mut connections = Vec::new();
    let mut next_id: i64 = 1_000_000;
    let mut id = || {
        next_id += 1;
        next_id
    };
    let (mut models, mut geometries, mut deformers, mut materials) = (0, 0, 0, 0);
    for o in objects {
        let (gid, mid, matid) = (id(), id(), id());
        geometries += 1;
        models += 1;
        materials += 1;
        objects_nodes.push(parent("Geometry", vec![P::I64(gid), named(o.name, "Geometry"), s("Mesh")], geometry_children(o.mesh)));
        objects_nodes.push(parent(
            "Model",
            vec![P::I64(mid), named(o.name, "Model"), s("Mesh")],
            vec![node("Version", vec![P::I32(232)]), parent("Properties70", vec![], vec![]), node("Shading", vec![P::I32(1)]), node("Culling", vec![s("CullingOff")])],
        ));
        let (r, g, b) = if o.mesh.has_paint() { (1.0, 1.0, 1.0) } else { (0.8, 0.8, 0.8) };
        objects_nodes.push(parent(
            "Material",
            vec![P::I64(matid), named(o.name, "Material"), s("")],
            vec![
                node("Version", vec![P::I32(102)]),
                node("ShadingModel", vec![s("phong")]),
                parent("Properties70", vec![], vec![prop70("DiffuseColor", "Color", "", "A", vec![P::F64(r), P::F64(g), P::F64(b)])]),
            ],
        ));
        connections.push(node("C", vec![s("OO"), P::I64(gid), P::I64(mid)]));
        connections.push(node("C", vec![s("OO"), P::I64(matid), P::I64(mid)]));
        connections.push(node("C", vec![s("OO"), P::I64(mid), P::I64(0)]));
        if !o.targets.is_empty() {
            let deformer = id();
            deformers += 1;
            objects_nodes.push(parent("Deformer", vec![P::I64(deformer), named(o.name, "Deformer"), s("BlendShape")], vec![node("Version", vec![P::I32(100)])]));
            connections.push(node("C", vec![s("OO"), P::I64(deformer), P::I64(gid)]));
            for t in &o.targets {
                let (shape, channel) = (id(), id());
                deformers += 1;
                geometries += 1;
                let changed: Vec<usize> = (0..t.delta.len()).filter(|&v| t.delta[v] != [0.0; 3]).collect();
                objects_nodes.push(parent(
                    "Geometry",
                    vec![P::I64(shape), named(t.name, "Geometry"), s("Shape")],
                    vec![
                        node("Version", vec![P::I32(100)]),
                        node("Indexes", vec![P::I32s(changed.iter().map(|&v| v as i32).collect())]),
                        node("Vertices", vec![P::F64s(changed.iter().flat_map(|&v| t.delta[v].iter().map(|&c| c as f64)).collect())]),
                        node("Normals", vec![P::F64s(vec![0.0; changed.len() * 3])]),
                    ],
                ));
                objects_nodes.push(parent(
                    "Deformer",
                    vec![P::I64(channel), named(t.name, "SubDeformer"), s("BlendShapeChannel")],
                    vec![node("Version", vec![P::I32(100)]), node("DeformPercent", vec![P::F64(t.weight as f64 * 100.0)]), node("FullWeights", vec![P::F64s(vec![100.0])])],
                ));
                connections.push(node("C", vec![s("OO"), P::I64(channel), P::I64(deformer)]));
                connections.push(node("C", vec![s("OO"), P::I64(shape), P::I64(channel)]));
            }
        }
    }
    let definition = |kind: &str, count: i32| parent("ObjectType", vec![s(kind)], vec![node("Count", vec![P::I32(count)])]);
    let mut defs = vec![node("Version", vec![P::I32(100)]), node("Count", vec![P::I32(1 + models + geometries + materials + deformers)]), definition("GlobalSettings", 1), definition("Model", models), definition("Geometry", geometries), definition("Material", materials)];
    if deformers > 0 {
        defs.push(definition("Deformer", deformers));
    }
    let top = vec![
        parent(
            "FBXHeaderExtension",
            vec![],
            vec![node("FBXHeaderVersion", vec![P::I32(1003)]), node("FBXVersion", vec![P::I32(VERSION as i32)]), node("Creator", vec![s(&format!("formsculptor {}", fsc_core::engine_version()))])],
        ),
        parent(
            "GlobalSettings",
            vec![],
            vec![
                node("Version", vec![P::I32(1000)]),
                parent(
                    "Properties70",
                    vec![],
                    vec![
                        prop70("UpAxis", "int", "Integer", "", vec![P::I32(1)]),
                        prop70("UpAxisSign", "int", "Integer", "", vec![P::I32(1)]),
                        prop70("FrontAxis", "int", "Integer", "", vec![P::I32(2)]),
                        prop70("FrontAxisSign", "int", "Integer", "", vec![P::I32(1)]),
                        prop70("CoordAxis", "int", "Integer", "", vec![P::I32(0)]),
                        prop70("CoordAxisSign", "int", "Integer", "", vec![P::I32(1)]),
                        prop70("UnitScaleFactor", "double", "Number", "", vec![P::F64(1.0)]),
                    ],
                ),
            ],
        ),
        node("Documents", vec![]),
        node("References", vec![]),
        parent("Definitions", vec![], defs),
        parent("Objects", vec![], objects_nodes),
        parent("Connections", vec![], connections),
    ];
    let mut out = MAGIC.to_vec();
    out.extend_from_slice(&VERSION.to_le_bytes());
    for n in &top {
        write_node(&mut out, n);
    }
    out.extend_from_slice(&[0u8; 13]);
    // The footer other readers expect: an id, padding to 16 bytes, the version and a closing magic.
    out.extend_from_slice(&[0xfa, 0xbc, 0xab, 0x09, 0xd0, 0xc8, 0xd4, 0x66, 0xb1, 0x76, 0xfb, 0x83, 0x1c, 0xf7, 0x26, 0x7e]);
    out.extend_from_slice(&[0u8; 4]);
    let pad = (16 - out.len() % 16) % 16;
    out.resize(out.len() + if pad == 0 { 16 } else { pad }, 0);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&[0u8; 120]);
    out.extend_from_slice(&[0xf8, 0x5a, 0x8c, 0x6a, 0xde, 0xf5, 0xd9, 0x7e, 0xec, 0xe9, 0x0c, 0xe3, 0x75, 0x8f, 0x29, 0x0b]);
    out
}

// ------------------------------------------------------------------ reading

#[derive(Debug, Clone)]
enum V {
    Int(i64),
    Float(f64),
    Text(String),
    Ints(Vec<i64>),
    Floats(Vec<f64>),
    Other,
}

#[derive(Debug)]
struct Rec {
    name: String,
    props: Vec<V>,
    children: Vec<Rec>,
}

impl Rec {
    fn child(&self, name: &str) -> Option<&Rec> {
        self.children.iter().find(|c| c.name == name)
    }
    fn floats(&self) -> Vec<f64> {
        match self.props.first() {
            Some(V::Floats(v)) => v.clone(),
            Some(V::Ints(v)) => v.iter().map(|&x| x as f64).collect(),
            _ => Vec::new(),
        }
    }
    fn ints(&self) -> Vec<i64> {
        match self.props.first() {
            Some(V::Ints(v)) => v.clone(),
            Some(V::Floats(v)) => v.iter().map(|&x| x as i64).collect(),
            _ => Vec::new(),
        }
    }
    fn text(&self, at: usize) -> String {
        match self.props.get(at) {
            Some(V::Text(t)) => t.clone(),
            _ => String::new(),
        }
    }
    fn id(&self) -> i64 {
        match self.props.first() {
            Some(V::Int(i)) => *i,
            _ => 0,
        }
    }
}

struct Reader<'a> {
    b: &'a [u8],
    wide: bool,
}

impl Reader<'_> {
    fn bytes(&self, at: usize, n: usize) -> Result<&[u8], String> {
        self.b.get(at..at + n).ok_or_else(|| "the FBX file is truncated".to_string())
    }
    fn u32(&self, at: usize) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.bytes(at, 4)?.try_into().expect("4")))
    }
    fn u64(&self, at: usize) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.bytes(at, 8)?.try_into().expect("8")))
    }

    fn array(&self, at: usize, size: usize) -> Result<(Vec<u8>, usize, usize), String> {
        let (count, encoding, len) = (self.u32(at)? as usize, self.u32(at + 4)?, self.u32(at + 8)? as usize);
        let raw = self.bytes(at + 12, len)?;
        let data = match encoding {
            0 => raw.to_vec(),
            1 => miniz_oxide::inflate::decompress_to_vec_zlib(raw).map_err(|e| format!("an FBX array does not inflate: {e:?}"))?,
            other => return Err(format!("unknown FBX array encoding {other}")),
        };
        if data.len() < count * size {
            return Err("an FBX array is shorter than its count".into());
        }
        Ok((data, count, at + 12 + len))
    }

    fn prop(&self, at: usize) -> Result<(V, usize), String> {
        let t = *self.b.get(at).ok_or("the FBX file is truncated")?;
        let at = at + 1;
        Ok(match t {
            b'Y' => (V::Int(i16::from_le_bytes(self.bytes(at, 2)?.try_into().expect("2")) as i64), at + 2),
            b'C' => (V::Int(self.bytes(at, 1)?[0] as i64), at + 1),
            b'I' => (V::Int(self.u32(at)? as i32 as i64), at + 4),
            b'L' => (V::Int(self.u64(at)? as i64), at + 8),
            b'F' => (V::Float(f32::from_le_bytes(self.bytes(at, 4)?.try_into().expect("4")) as f64), at + 4),
            b'D' => (V::Float(f64::from_le_bytes(self.bytes(at, 8)?.try_into().expect("8"))), at + 8),
            b'S' | b'R' => {
                let n = self.u32(at)? as usize;
                let raw = self.bytes(at + 4, n)?;
                (if t == b'S' { V::Text(String::from_utf8_lossy(raw).into_owned()) } else { V::Other }, at + 4 + n)
            }
            b'f' | b'd' | b'i' | b'l' | b'b' => {
                let size = match t {
                    b'd' | b'l' => 8,
                    b'b' => 1,
                    _ => 4,
                };
                let (data, count, next) = self.array(at, size)?;
                let v = match t {
                    b'f' => V::Floats(data.chunks_exact(4).take(count).map(|c| f32::from_le_bytes(c.try_into().expect("4")) as f64).collect()),
                    b'd' => V::Floats(data.chunks_exact(8).take(count).map(|c| f64::from_le_bytes(c.try_into().expect("8"))).collect()),
                    b'i' => V::Ints(data.chunks_exact(4).take(count).map(|c| i32::from_le_bytes(c.try_into().expect("4")) as i64).collect()),
                    b'l' => V::Ints(data.chunks_exact(8).take(count).map(|c| i64::from_le_bytes(c.try_into().expect("8"))).collect()),
                    _ => V::Ints(data.iter().take(count).map(|&x| x as i64).collect()),
                };
                (v, next)
            }
            other => return Err(format!("unknown FBX property type {:?}", other as char)),
        })
    }

    /// One record at `at`; None for the null record that closes a list. Returns the offset after it.
    fn record(&self, at: usize, depth: usize) -> Result<(Option<Rec>, usize), String> {
        if depth > 64 {
            return Err("the FBX nesting is too deep".into());
        }
        let (end, count, head) = if self.wide { (self.u64(at)? as usize, self.u64(at + 8)? as usize, 24) } else { (self.u32(at)? as usize, self.u32(at + 4)? as usize, 12) };
        let name_len = *self.b.get(at + head).ok_or("the FBX file is truncated")? as usize;
        if end == 0 {
            return Ok((None, at + head + 1));
        }
        if end > self.b.len() || end <= at {
            return Err("an FBX record points outside the file".into());
        }
        let name = String::from_utf8_lossy(self.bytes(at + head + 1, name_len)?).into_owned();
        let mut p = at + head + 1 + name_len;
        let mut props = Vec::with_capacity(count.min(64));
        for _ in 0..count {
            let (v, next) = self.prop(p)?;
            props.push(v);
            p = next;
        }
        let mut children = Vec::new();
        while p < end {
            let (child, next) = self.record(p, depth + 1)?;
            p = next;
            match child {
                Some(c) => children.push(c),
                None => break,
            }
        }
        Ok((Some(Rec { name, props, children }), end))
    }
}

fn lcl(model: &Rec) -> Mat4 {
    let mut t = Vec3::ZERO;
    let mut r = Vec3::ZERO;
    let mut s = Vec3::ONE;
    if let Some(p70) = model.child("Properties70") {
        for p in p70.children.iter().filter(|p| p.name == "P") {
            let v = |i: usize| match p.props.get(i) {
                Some(V::Float(f)) => *f as f32,
                Some(V::Int(n)) => *n as f32,
                _ => 0.0,
            };
            let three = Vec3::new(v(4), v(5), v(6));
            match p.text(0).as_str() {
                "Lcl Translation" => t = three,
                "Lcl Rotation" => r = three,
                "Lcl Scaling" => s = three,
                _ => {}
            }
        }
    }
    let q = Quat::from_euler(EulerRot::ZYX, r.z.to_radians(), r.y.to_radians(), r.x.to_radians());
    Mat4::from_scale_rotation_translation(s, q, t)
}

/// Joins vertices at the same position (many exporters split them per corner for normals or UVs), keeping the first
/// one's paint, so an imported mesh is one connected surface to sculpt.
fn weld(mesh: Mesh) -> Mesh {
    let mut index: std::collections::HashMap<[u32; 3], u32> = std::collections::HashMap::with_capacity(mesh.vertex_count());
    let mut remap = Vec::with_capacity(mesh.vertex_count());
    let mut positions = Vec::new();
    let mut paint = Vec::new();
    for (v, p) in mesh.positions.iter().enumerate() {
        let k = [p[0].to_bits(), p[1].to_bits(), p[2].to_bits()];
        let i = *index.entry(k).or_insert_with(|| {
            positions.push(*p);
            if mesh.has_paint() {
                paint.push(mesh.paint[v]);
            }
            positions.len() as u32 - 1
        });
        remap.push(i);
    }
    if positions.len() == mesh.vertex_count() {
        return mesh;
    }
    let indices = mesh.indices.iter().map(|&i| remap[i as usize]).collect();
    let mut out = Mesh::new(positions, indices);
    out.paint = paint;
    out
}

/// Reads the meshes of a binary FBX 7.x file, each placed by its model's local transform and named after it, with
/// vertex colours as paint. Polygons with more than three corners are split into fans.
pub fn read_fbx(bytes: &[u8]) -> Result<Vec<Named>, String> {
    if bytes.starts_with(b"; FBX") {
        return Err("this is an ASCII FBX; export it as binary FBX, glTF or OBJ".into());
    }
    if !bytes.starts_with(&MAGIC[..20]) || bytes.len() < 27 {
        return Err("not a binary FBX file".into());
    }
    let version = u32::from_le_bytes(bytes[23..27].try_into().expect("4"));
    let r = Reader { b: bytes, wide: version >= 7500 };
    let mut at = 27;
    let mut top = Vec::new();
    while at < bytes.len() {
        let (rec, next) = r.record(at, 0)?;
        match rec {
            Some(rec) => top.push(rec),
            None => break,
        }
        at = next;
    }
    let objects = top.iter().find(|n| n.name == "Objects").ok_or("the FBX has no Objects")?;
    let models: Vec<&Rec> = objects.children.iter().filter(|n| n.name == "Model").collect();
    let mut parent_of: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
    if let Some(conns) = top.iter().find(|n| n.name == "Connections") {
        for c in conns.children.iter().filter(|c| c.name == "C" && c.text(0) == "OO") {
            if let (Some(V::Int(child)), Some(V::Int(par))) = (c.props.get(1), c.props.get(2)) {
                parent_of.entry(*child).or_insert(*par);
            }
        }
    }
    let world = |mut id: i64| -> Mat4 {
        let mut m = Mat4::IDENTITY;
        for _ in 0..64 {
            let Some(model) = models.iter().find(|n| n.id() == id) else { break };
            m = lcl(model) * m;
            match parent_of.get(&id) {
                Some(&p) if p != 0 => id = p,
                _ => break,
            }
        }
        m
    };
    let mut out = Vec::new();
    for g in objects.children.iter().filter(|n| n.name == "Geometry" && n.text(2) == "Mesh") {
        let verts = g.child("Vertices").map(Rec::floats).unwrap_or_default();
        let polys = g.child("PolygonVertexIndex").map(Rec::ints).unwrap_or_default();
        if verts.len() < 9 || polys.len() < 3 {
            continue;
        }
        let model = parent_of.get(&g.id()).copied().unwrap_or(0);
        let m = world(model);
        let positions: Vec<[f32; 3]> = verts.chunks_exact(3).map(|c| m.transform_point3(Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32)).to_array()).collect();
        let n = positions.len() as i64;
        let mut indices = Vec::new();
        let mut corners: Vec<u32> = Vec::new();
        let mut poly_corner_vertex = Vec::with_capacity(polys.len());
        for &raw in &polys {
            let (v, last) = if raw < 0 { (-raw - 1, true) } else { (raw, false) };
            if v >= n {
                return Err("an FBX polygon points past the vertices".into());
            }
            corners.push(v as u32);
            poly_corner_vertex.push(v as u32);
            if last {
                for k in 1..corners.len().saturating_sub(1) {
                    indices.extend_from_slice(&[corners[0], corners[k], corners[k + 1]]);
                }
                corners.clear();
            }
        }
        let mut mesh = Mesh::new(positions, indices);
        if let Some(col) = g.child("LayerElementColor") {
            let values = col.child("Colors").map(Rec::floats).unwrap_or_default();
            let index = col.child("ColorIndex").map(Rec::ints);
            let mapping = col.child("MappingInformationType").map(|r| r.text(0)).unwrap_or_default();
            let lookup = |k: usize| -> Option<[f32; 3]> {
                let i = match &index {
                    Some(ix) => *ix.get(k)? as usize,
                    None => k,
                };
                let c = values.get(i * 4..i * 4 + 3)?;
                Some([c[0] as f32, c[1] as f32, c[2] as f32])
            };
            let mut sum = vec![[0.0f32; 4]; mesh.vertex_count()];
            if mapping == "ByVertice" || mapping == "ByVertex" {
                for (v, s) in sum.iter_mut().enumerate() {
                    if let Some(c) = lookup(v) {
                        *s = [c[0], c[1], c[2], 1.0];
                    }
                }
            } else {
                for (k, &v) in poly_corner_vertex.iter().enumerate() {
                    if let Some(c) = lookup(k) {
                        let s = &mut sum[v as usize];
                        *s = [s[0] + c[0], s[1] + c[1], s[2] + c[2], s[3] + 1.0];
                    }
                }
            }
            if sum.iter().any(|s| s[3] > 0.0) {
                mesh.paint = sum.iter().map(|s| if s[3] > 0.0 { fsc_core::paint_from5(&[s[0] / s[3], s[1] / s[3], s[2] / s[3], 0.5, 0.0]) } else { fsc_core::DEFAULT_PAINT }).collect();
            }
        }
        let mesh = weld(mesh);
        let name = models.iter().find(|n| n.id() == model).map(|n| n.text(1)).unwrap_or_else(|| g.text(1));
        let name = name.split('\u{0}').next().unwrap_or("Imported").to_string();
        out.push((if name.is_empty() { "Imported".into() } else { name }, mesh));
    }
    if out.is_empty() {
        return Err("the FBX has no meshes".into());
    }
    Ok(out)
}
