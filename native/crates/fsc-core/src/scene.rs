use crate::bvh::Bvh;
use crate::mesh::Mesh;
use crate::layers::Layers;
use crate::multires::Multires;
use crate::texture::Texture;
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
    /// Paint kept per texel in UV space (tile 1001); dropped whenever the UVs go or change.
    pub texture: Option<Texture>,
    /// Baked maps per UDIM tile (tile number, maps), ascending by tile; empty unless a udim bake kept them.
    pub tile_maps: Vec<(u32, Maps)>,
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
    pub selection_sets: Vec<crate::SelectionSet>,
    /// The object or group this one hangs under in the scene hierarchy; moving a parent moves its children. An id that
    /// no longer names anything (the parent was deleted) counts as none, so undoing the delete hangs it back.
    pub parent: Option<ObjectId>,
}

/// An empty node of the scene hierarchy: it has no mesh, only a name and a place, and exists to hold and move the
/// objects under it together. Groups share the objects' id numbers, so a parent id names either.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub id: ObjectId,
    pub name: String,
    pub parent: Option<ObjectId>,
}

/// The scene hierarchy alone (every object's parent and the groups), swapped whole by an undo step.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Hierarchy {
    pub parents: Vec<(ObjectId, Option<ObjectId>)>,
    pub groups: Vec<Group>,
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
    /// How the surface is put over what is behind it.
    pub blend: MaterialBlend,
    /// 0 to 1: for Dithered, the share of the surface drawn; for Additive, how much light it adds. Opaque ignores it.
    pub opacity: f32,
    /// Smooth (normals interpolated) or faceted (each face shaded on its own) shading; None follows the view's
    /// setting, as Nomad's per-material Smooth Shading Auto does.
    pub smooth: Option<bool>,
}

impl Default for Material {
    fn default() -> Material {
        Material { subsurface: 0.0, subsurface_color: [1.0, 0.35, 0.25], subsurface_radius: 0.2, transmission: 0.0, ior: 1.45, blend: MaterialBlend::Opaque, opacity: 1.0, smooth: None }
    }
}

impl Material {
    /// The smooth shading setting's name: auto, on or off.
    pub fn smooth_name(&self) -> &'static str {
        match self.smooth {
            None => "auto",
            Some(true) => "on",
            Some(false) => "off",
        }
    }

    /// Parses auto, on or off.
    pub fn parse_smooth(name: &str) -> Option<Option<bool>> {
        match name {
            "auto" => Some(None),
            "on" => Some(Some(true)),
            "off" => Some(Some(false)),
            _ => None,
        }
    }
}

/// A material's blending: how a surface covers what is behind it (paint layers' Blend is a separate thing, how a
/// paint layer mixes with those under it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaterialBlend {
    /// Covers what is behind it.
    #[default]
    Opaque,
    /// See-through by leaving out a fine pattern of pixels (a share of 1 - opacity): needs no sorting, keeps depth and
    /// shadows, and grows smooth with samples in a path render.
    Dithered,
    /// Adds its light to what is behind it and hides nothing: glows, holograms, light beams. Casts no shadow.
    Additive,
}

impl MaterialBlend {
    pub const NAMES: [&'static str; 3] = ["opaque", "dithered", "additive"];

    pub fn name(self) -> &'static str {
        match self {
            MaterialBlend::Opaque => "opaque",
            MaterialBlend::Dithered => "dithered",
            MaterialBlend::Additive => "additive",
        }
    }

    pub fn parse(name: &str) -> Option<MaterialBlend> {
        Some(match name {
            "opaque" => MaterialBlend::Opaque,
            "dithered" => MaterialBlend::Dithered,
            "additive" => MaterialBlend::Additive,
            _ => return None,
        })
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
    /// The glow map (paint colour times emissive), empty when nothing glows.
    pub emissive_png: Vec<u8>,
    pub source: String,
    /// True when the colour map holds opacity below 1, so a glTF material needs alpha blending (set for maps made from a texture).
    pub translucent: bool,
}

impl Object {
    pub fn new(id: ObjectId, name: String, mesh: Mesh) -> Object {
        let bvh = Bvh::build(&mesh);
        let topology = Topology::build(&mesh);
        let mask = vec![0.0; mesh.vertex_count()];
        Object { id, name, mesh, bvh, topology, mask, visible: true, revision: 1, multires: None, layers: Layers::default(), maps: None, texture: None, tile_maps: Vec::new(), folder: String::new(), paint_layers: crate::PaintLayers::default(), primitive: None, instance: None, material: Material::default(), selection_sets: Vec::new(), parent: None }
    }

    /// Drops the baked maps, the single set and every UDIM tile's, and the texture, which every change of UVs ends.
    pub fn clear_maps(&mut self) {
        self.maps = None;
        self.tile_maps.clear();
        self.texture = None;
    }

    /// True when the mesh has a UV for every corner.
    pub fn has_uvs(&self) -> bool {
        !self.mesh.indices.is_empty() && self.mesh.uvs.len() == self.mesh.indices.len()
    }

    /// The texture, while the mesh still has the UVs it was made for (a count check; UV edits drop it).
    pub fn live_texture(&self) -> Option<&Texture> {
        self.texture.as_ref().filter(|_| self.has_uvs())
    }

    pub fn rebuild(&mut self) {
        if !self.has_uvs() {
            self.texture = None;
        }
        self.bvh = Bvh::build(&self.mesh);
        self.topology = Topology::build(&self.mesh);
        self.mask.resize(self.mesh.vertex_count(), 0.0);
        self.revision += 1;
    }

    pub fn retopologize(&mut self) {
        if !self.has_uvs() {
            self.texture = None;
        }
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

/// A camera view kept by name with the scene and saved in projects: the orbit target, the eye's distance from it, its
/// direction as yaw and pitch in degrees (get_view's convention) and whether the view is orthographic.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedView {
    pub name: String,
    pub target: [f32; 3],
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub ortho: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub objects: Vec<Object>,
    pub active: Option<ObjectId>,
    next_id: ObjectId,
    /// Named camera views, in the order they were saved (saved in projects, not in the undo history).
    pub views: Vec<SavedView>,
    /// Empty hierarchy nodes, in the order they were made.
    pub groups: Vec<Group>,
    /// Real-world scale: millimetres in one scene unit, once set (set_real_size). Print check measures by it and STL
    /// and 3MF exports are written in millimetres by it; unset, they read as before.
    pub mm_per_unit: Option<f32>,
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
        self.groups.clear();
        self.active = None;
    }

    /// Makes an empty group, under `parent` when given, and returns its id (from the objects' numbering).
    pub fn add_group(&mut self, name: impl Into<String>, parent: Option<ObjectId>) -> ObjectId {
        self.next_id += 1;
        let id = self.next_id;
        self.groups.push(Group { id, name: name.into(), parent });
        id
    }

    pub fn group(&self, id: ObjectId) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// Whether `id` names an object or a group.
    pub fn exists(&self, id: ObjectId) -> bool {
        self.get(id).is_some() || self.group(id).is_some()
    }

    /// The node `id` hangs under, if that node still exists.
    pub fn parent_of(&self, id: ObjectId) -> Option<ObjectId> {
        let p = self.get(id).map(|o| o.parent).or_else(|| self.group(id).map(|g| g.parent)).flatten()?;
        self.exists(p).then_some(p)
    }

    /// Whether `ancestor` is `id` or above it in the hierarchy.
    pub fn is_ancestor(&self, ancestor: ObjectId, id: ObjectId) -> bool {
        let mut at = Some(id);
        // The walk is bounded by the node count, so a cycle (which set_parent refuses) cannot hang it.
        for _ in 0..=self.objects.len() + self.groups.len() {
            match at {
                Some(a) if a == ancestor => return true,
                Some(a) => at = self.parent_of(a),
                None => return false,
            }
        }
        false
    }

    /// The objects below `id` (not `id` itself), in scene order.
    pub fn descendants(&self, id: ObjectId) -> Vec<ObjectId> {
        self.objects.iter().filter(|o| o.id != id && self.is_ancestor(id, o.id)).map(|o| o.id).collect()
    }

    /// Hangs `id` under `parent` (None: at the top). Refuses a parent that is the node itself or below it.
    pub fn set_parent(&mut self, id: ObjectId, parent: Option<ObjectId>) -> Result<(), String> {
        if let Some(p) = parent {
            if !self.exists(p) {
                return Err(format!("no object or group {p}"));
            }
            if self.is_ancestor(id, p) {
                return Err(format!("{p} is {id} or below it, so it cannot be its parent"));
            }
        }
        if let Some(o) = self.get_mut(id) {
            o.parent = parent;
            o.revision += 1;
        } else if let Some(g) = self.groups.iter_mut().find(|g| g.id == id) {
            g.parent = parent;
        } else {
            return Err(format!("no object or group {id}"));
        }
        Ok(())
    }

    /// The hierarchy as it stands, for an undo step.
    pub fn hierarchy(&self) -> Hierarchy {
        Hierarchy { parents: self.objects.iter().map(|o| (o.id, o.parent)).collect(), groups: self.groups.clone() }
    }

    /// Puts a hierarchy back and returns the one it replaced (objects made or removed since keep theirs).
    pub fn swap_hierarchy(&mut self, h: Hierarchy) -> Hierarchy {
        let old = self.hierarchy();
        for (id, parent) in h.parents {
            if let Some(o) = self.get_mut(id) {
                if o.parent != parent {
                    o.parent = parent;
                    o.revision += 1;
                }
            }
        }
        self.groups = h.groups;
        old
    }

    pub fn unique_name(&self, base: &str) -> String {
        if !self.objects.iter().any(|o| o.name == base) {
            return base.to_string();
        }
        (2..).map(|n| format!("{base} {n}")).find(|c| !self.objects.iter().any(|o| &o.name == c)).unwrap_or_else(|| base.to_string())
    }
}
