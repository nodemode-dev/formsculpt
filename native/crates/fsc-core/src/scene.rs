use crate::bvh::Bvh;
use crate::mesh::Mesh;
use crate::layers::Layers;
use crate::multires::Multires;
use crate::topology::Topology;

pub type ObjectId = u32;

#[derive(Debug, Clone)]
pub struct Object {
    pub id: ObjectId,
    pub name: String,
    pub mesh: Mesh,
    pub bvh: Bvh,
    pub topology: Topology,
    pub mask: Vec<f32>,
    pub visible: bool,
    pub revision: u64,
    pub multires: Option<Multires>,
    pub layers: Layers,
    pub maps: Option<Maps>,
    /// The folder the object is filed under in the scene list (one level, as ZBrush's SubTool folders); empty: none.
    pub folder: String,
    /// Paint layers composited into `mesh.paint` (empty: the paint is plain).
    pub paint_layers: crate::PaintLayers,
    /// A primitive still open to edit: the arguments it was made with (JSON) and the revision it had then; any other
    /// change raises the revision and so fixes the shape.
    pub primitive: Option<(String, u64)>,
    /// A linked copy: rebuilt from its source whenever the source changes, until it is changed itself.
    pub instance: Option<Instance>,
    /// How light passes into and through the surface, beyond its painted colour, roughness and metalness.
    pub material: Material,
}

/// An object's light transport in lit shading: subsurface scattering (skin, wax, jade) and transmission with refraction
/// (glass, water, resin). Zero subsurface and transmission is an ordinary opaque surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    /// 0 to 1: how much light enters the surface and comes back out tinted.
    pub subsurface: f32,
    /// The colour light picks up inside (linear RGB, 0 to 1).
    pub subsurface_color: [f32; 3],
    /// How far light travels inside, in world units; thin parts glow through when lit from behind.
    pub subsurface_radius: f32,
    /// 0 to 1: how much light passes through, refracted.
    pub transmission: f32,
    /// Index of refraction, 1 to 2.5 (water 1.33, glass 1.5, diamond 2.4).
    pub ior: f32,
}

impl Default for Material {
    fn default() -> Material {
        Material { subsurface: 0.0, subsurface_color: [1.0, 0.35, 0.25], subsurface_radius: 0.2, transmission: 0.0, ior: 1.45 }
    }
}

#[derive(Debug, Clone)]
pub struct Instance {
    pub source: ObjectId,
    /// The copy's transform from the source, column-major.
    pub matrix: [f32; 16],
    /// The source's revision when this copy was last rebuilt, and this copy's revision then.
    pub source_revision: u64,
    pub own_revision: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Maps {
    pub size: usize,
    pub color_png: Vec<u8>,
    pub mr_png: Vec<u8>,
    pub normal_png: Vec<u8>,
    pub displacement_exr: Vec<u8>,
    pub source: String,
}

impl Object {
    pub fn new(id: ObjectId, name: String, mesh: Mesh) -> Object {
        let bvh = Bvh::build(&mesh);
        let topology = Topology::build(&mesh);
        let mask = vec![0.0; mesh.vertex_count()];
        Object { id, name, mesh, bvh, topology, mask, visible: true, revision: 1, multires: None, layers: Layers::default(), maps: None, folder: String::new(), paint_layers: crate::PaintLayers::default(), primitive: None, instance: None, material: Material::default() }
    }

    pub fn rebuild(&mut self) {
        self.bvh = Bvh::build(&self.mesh);
        self.topology = Topology::build(&self.mesh);
        self.mask.resize(self.mesh.vertex_count(), 0.0);
        self.revision += 1;
    }

    pub fn retopologize(&mut self) {
        self.mesh.normals.resize(self.mesh.vertex_count(), [0.0, 1.0, 0.0]);
        if self.mesh.has_paint() {
            self.mesh.ensure_paint();
        }
        self.mask.resize(self.mesh.vertex_count(), 0.0);
        self.topology = Topology::build(&self.mesh);
        let all: Vec<u32> = (0..self.mesh.vertex_count() as u32).collect();
        self.topology.update_normals(&mut self.mesh, &all);
        self.bvh = Bvh::build(&self.mesh);
    }

    pub fn replace_mesh(&mut self, mesh: Mesh, mask: Vec<f32>) -> (Mesh, Vec<f32>) {
        let old_mesh = std::mem::replace(&mut self.mesh, mesh);
        let old_mask = std::mem::replace(&mut self.mask, mask);
        self.rebuild();
        (old_mesh, old_mask)
    }

    pub fn refit_vertices(&mut self, verts: &[u32]) {
        let topology = &self.topology;
        let tris = verts.iter().flat_map(|&v| topology.faces_of(v).iter().copied());
        self.bvh.refit_triangles(&self.mesh, tris);
    }

    pub fn mask_bytes(&self) -> Vec<u8> {
        self.mask.iter().flat_map(|m| m.to_le_bytes()).collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub objects: Vec<Object>,
    pub active: Option<ObjectId>,
    next_id: ObjectId,
}

impl Scene {
    pub fn add(&mut self, name: impl Into<String>, mesh: Mesh) -> ObjectId {
        self.next_id += 1;
        let id = self.next_id;
        self.objects.push(Object::new(id, name.into(), mesh));
        self.active = Some(id);
        id
    }

    pub fn insert(&mut self, at: usize, object: Object) {
        self.next_id = self.next_id.max(object.id);
        let id = object.id;
        self.objects.insert(at.min(self.objects.len()), object);
        self.active = Some(id);
    }

    pub fn high_water(&self) -> ObjectId {
        self.next_id
    }

    pub fn reserve(&mut self, id: ObjectId) {
        self.next_id = self.next_id.max(id);
    }

    pub fn index_of(&self, id: ObjectId) -> Option<usize> {
        self.objects.iter().position(|o| o.id == id)
    }

    pub fn get(&self, id: ObjectId) -> Option<&Object> {
        self.objects.iter().find(|o| o.id == id)
    }

    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut Object> {
        self.objects.iter_mut().find(|o| o.id == id)
    }

    pub fn remove(&mut self, id: ObjectId) -> Option<Object> {
        let at = self.objects.iter().position(|o| o.id == id)?;
        let removed = self.objects.remove(at);
        if self.active == Some(id) {
            self.active = self.objects.last().map(|o| o.id);
        }
        Some(removed)
    }

    pub fn clear(&mut self) {
        self.objects.clear();
        self.active = None;
    }

    pub fn unique_name(&self, base: &str) -> String {
        if !self.objects.iter().any(|o| o.name == base) {
            return base.to_string();
        }
        (2..).map(|n| format!("{base} {n}")).find(|c| !self.objects.iter().any(|o| &o.name == c)).unwrap_or_else(|| base.to_string())
    }
}
