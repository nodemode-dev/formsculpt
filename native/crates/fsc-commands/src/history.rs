use fsc_core::{Layers, Mesh, Multires, Object, ObjectId, Scene, Stencil};
use fsc_sculpt::VertexEdit;

const LIMIT: usize = 200;

#[derive(Debug)]
pub enum Edit {
    Vertices { label: String, object: ObjectId, edit: VertexEdit },
    Topology { label: String, object: ObjectId, mesh: Mesh, mask: Vec<f32> },
    Add { id: ObjectId, index: usize, parked: Option<Object> },
    Delete { id: ObjectId, index: usize, parked: Option<Object> },
    Clear { parked: Vec<Object>, active: Option<ObjectId> },
    Divide { object: ObjectId, count: usize, parked: Vec<(Mesh, Stencil)> },
    Level { object: ObjectId, from: usize, to: usize },
    Levels { label: String, object: ObjectId, multires: Option<Multires> },
    Scene { label: String, objects: Vec<Object>, active: Option<ObjectId> },
    Rename { object: ObjectId, name: String },
    LayerState { label: String, object: ObjectId, layers: Layers, positions: Vec<[f32; 3]> },
    LayerStroke { label: String, object: ObjectId, layer: usize, edit: VertexEdit },
}

impl Edit {
    pub fn label(&self) -> String {
        match self {
            Edit::Vertices { label, .. }
            | Edit::Topology { label, .. }
            | Edit::Levels { label, .. }
            | Edit::Scene { label, .. }
            | Edit::LayerState { label, .. }
            | Edit::LayerStroke { label, .. } => label.clone(),
            Edit::Rename { .. } => "Rename".into(),
            Edit::Divide { .. } => "Divide".into(),
            Edit::Level { to, .. } => format!("Level {to}"),
            Edit::Add { .. } => "Add Object".into(),
            Edit::Delete { .. } => "Delete Object".into(),
            Edit::Clear { .. } => "New Scene".into(),
        }
    }

    fn apply(&mut self, scene: &mut Scene, forward: bool) -> Result<(), String> {
        match self {
            Edit::Vertices { object, edit, .. } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                edit.apply(o, forward);
                scene.active = Some(*object);
            }
            Edit::Topology { object, mesh, mask, .. } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                let (old_mesh, old_mask) = o.replace_mesh(std::mem::take(mesh), std::mem::take(mask));
                *mesh = old_mesh;
                *mask = old_mask;
                scene.active = Some(*object);
            }
            Edit::Add { id, index, parked } | Edit::Delete { id, index, parked } => {
                if let Some(o) = parked.take() {
                    scene.insert(*index, o);
                } else {
                    *index = scene.index_of(*id).ok_or("the object is already gone")?;
                    *parked = scene.remove(*id);
                }
            }
            Edit::Divide { object, count, parked } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                if forward {
                    for (mesh, stencil) in parked.drain(..).rev() {
                        o.divide(mesh, stencil)?;
                    }
                } else {
                    for _ in 0..*count {
                        parked.push(o.undivide()?);
                    }
                }
                scene.active = Some(*object);
            }
            Edit::Level { object, from, to } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                o.set_level(if forward { *to } else { *from })?;
                scene.active = Some(*object);
            }
            Edit::Levels { object, multires, .. } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                std::mem::swap(&mut o.multires, multires);
                o.revision += 1;
                scene.active = Some(*object);
            }
            Edit::Scene { objects, active, .. } => {
                std::mem::swap(&mut scene.objects, objects);
                std::mem::swap(&mut scene.active, active);
                if let Some(top) = scene.objects.iter().map(|o| o.id).max() {
                    scene.reserve(top);
                }
            }
            Edit::Rename { object, name } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                std::mem::swap(&mut o.name, name);
                o.revision += 1;
            }
            Edit::LayerState { object, layers, positions, .. } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                if positions.len() != o.mesh.vertex_count() {
                    return Err("the object's vertex count changed since this layer step".into());
                }
                std::mem::swap(&mut o.layers, layers);
                std::mem::swap(&mut o.mesh.positions, positions);
                o.mesh.recompute_normals();
                o.bvh.refit(&o.mesh);
                o.revision += 1;
                scene.active = Some(*object);
            }
            Edit::LayerStroke { object, layer, edit, .. } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                edit.apply(o, forward);
                o.record_into_layer(*layer, &edit.indices, &edit.before, &edit.after, if forward { 1.0 } else { -1.0 });
                scene.active = Some(*object);
            }
            Edit::Clear { parked, active } => {
                if forward {
                    *active = scene.active;
                    *parked = std::mem::take(&mut scene.objects);
                    scene.active = None;
                } else {
                    scene.objects = std::mem::take(parked);
                    scene.active = *active;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Edit>,
    redo: Vec<Edit>,
}

impl History {
    pub fn push(&mut self, edit: Edit) {
        self.redo.clear();
        self.undo.push(edit);
        if self.undo.len() > LIMIT {
            self.undo.remove(0);
        }
    }

    pub fn undo(&mut self, scene: &mut Scene) -> Result<Option<String>, String> {
        let Some(mut edit) = self.undo.pop() else { return Ok(None) };
        let label = edit.label();
        edit.apply(scene, false)?;
        self.redo.push(edit);
        Ok(Some(label))
    }

    pub fn redo(&mut self, scene: &mut Scene) -> Result<Option<String>, String> {
        let Some(mut edit) = self.redo.pop() else { return Ok(None) };
        let label = edit.label();
        edit.apply(scene, true)?;
        self.undo.push(edit);
        Ok(Some(label))
    }

    pub fn depth(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }
}
