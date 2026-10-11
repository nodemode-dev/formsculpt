use crate::bvh::Bvh;
use crate::mesh::Mesh;

pub type ObjectId = u32;

#[derive(Debug, Clone)]
pub struct Object {
    pub id: ObjectId,
    pub name: String,
    pub mesh: Mesh,
    pub bvh: Bvh,
    pub visible: bool,
    pub revision: u64,
}

impl Object {
    pub fn rebuild(&mut self) {
        self.bvh = Bvh::build(&self.mesh);
        self.revision += 1;
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
        let bvh = Bvh::build(&mesh);
        self.objects.push(Object { id, name: name.into(), mesh, bvh, visible: true, revision: 1 });
        self.active = Some(id);
        id
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
