use fsc_core::{Layers, Mesh, Multires, Object, ObjectId, Paint, PaintLayers, Scene, Stencil};
use fsc_sculpt::VertexEdit;

/// Steps kept until the person sets another limit.
pub const DEFAULT_STEPS: usize = 200;
/// The most steps a limit may keep.
pub const MAX_STEPS: usize = 10_000;

/// Where the view was when a step was made, as the window sent it: the orbit target, the eye's distance, its
/// direction as yaw and pitch in degrees (get_view's convention) and whether the view was orthographic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepCamera {
    pub target: [f32; 3],
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub ortho: bool,
}

#[derive(Debug)]
struct Step {
    edit: Edit,
    camera: Option<StepCamera>,
}

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
    LayerState { label: String, object: ObjectId, layers: Layers, positions: Vec<[f32; 3]>, paint: Vec<Paint>, mask: Vec<f32> },
    LayerStroke { label: String, object: ObjectId, layer: usize, edit: VertexEdit },
    /// A paint layer step (add, delete, settings, merge, flatten): the whole stack and the composite swap back.
    PaintLayers { label: String, object: ObjectId, layers: PaintLayers, paint: Vec<Paint> },
    /// The texels one texture stroke or fill changed.
    Texels { label: String, object: ObjectId, edit: fsc_core::TextureEdit },
    /// The texture made or removed: the other state swaps back.
    TextureState { label: String, object: ObjectId, texture: Option<fsc_core::Texture> },
    /// A material change (subsurface, transmission): the previous material swaps back.
    Material { object: ObjectId, material: fsc_core::Material },
    /// Several channels (groups, mask, paint, sculpt layers, UVs) swapped as one step: selection set restores, pastes, clears.
    Channels { label: String, object: ObjectId, state: fsc_core::ChannelState },
}

impl Edit {
    pub fn label(&self) -> String {
        match self {
            Edit::Vertices { label, .. }
            | Edit::Topology { label, .. }
            | Edit::Levels { label, .. }
            | Edit::Scene { label, .. }
            | Edit::LayerState { label, .. }
            | Edit::LayerStroke { label, .. }
            | Edit::PaintLayers { label, .. }
            | Edit::Texels { label, .. }
            | Edit::TextureState { label, .. }
            | Edit::Channels { label, .. } => label.clone(),
            Edit::Rename { .. } => "Rename".into(),
            Edit::Material { .. } => "Material".into(),
            Edit::Divide { .. } => "Divide".into(),
            Edit::Level { to, .. } => format!("Level {to}"),
            Edit::Add { .. } => "Add Object".into(),
            Edit::Delete { .. } => "Delete Object".into(),
            Edit::Clear { .. } => "New Scene".into(),
        }
    }

    /// The memory this step keeps (its arrays, see weigh.rs). Undo and redo swap what a step holds, so the figure is
    /// taken when asked rather than stored.
    pub fn bytes(&self) -> usize {
        use crate::weigh;
        match self {
            Edit::Vertices { edit, .. } | Edit::LayerStroke { edit, .. } => weigh::vertex_edit(edit),
            Edit::Topology { mesh, mask, .. } => weigh::mesh(mesh) + std::mem::size_of_val(mask.as_slice()),
            Edit::Add { parked, .. } | Edit::Delete { parked, .. } => parked.as_ref().map_or(0, weigh::object),
            Edit::Clear { parked, .. } | Edit::Scene { objects: parked, .. } => parked.iter().map(weigh::object).sum(),
            Edit::Divide { parked, .. } => parked.iter().map(|(m, s)| weigh::mesh(m) + weigh::stencil(s)).sum(),
            Edit::Level { .. } => 0,
            Edit::Levels { multires, .. } => weigh::multires(multires),
            Edit::Rename { name, .. } => name.len(),
            Edit::LayerState { layers, positions, paint, mask, .. } => {
                weigh::layers(layers) + std::mem::size_of_val(positions.as_slice()) + std::mem::size_of_val(paint.as_slice()) + std::mem::size_of_val(mask.as_slice())
            }
            Edit::PaintLayers { layers, paint, .. } => weigh::paint_layers(layers) + std::mem::size_of_val(paint.as_slice()),
            Edit::Texels { edit, .. } => weigh::texture_edit(edit),
            Edit::TextureState { texture, .. } => weigh::texture(texture),
            Edit::Material { .. } => std::mem::size_of::<fsc_core::Material>(),
            Edit::Channels { state, .. } => weigh::channels(state),
        }
    }

    fn apply(&mut self, scene: &mut Scene, forward: bool) -> Result<(), String> {
        match self {
            Edit::Vertices { object, edit, .. } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                edit.apply(o, forward);
                scene.active = Some(*object);
            }
            Edit::Texels { object, edit, .. } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                let texture = o.texture.as_mut().ok_or("the object's texture is gone")?;
                edit.apply(texture, forward);
                o.revision += 1;
                scene.active = Some(*object);
            }
            Edit::TextureState { object, texture, .. } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                std::mem::swap(&mut o.texture, texture);
                if let Some(t) = &mut o.texture {
                    t.revision += 1;
                }
                o.revision += 1;
                scene.active = Some(*object);
            }
            Edit::PaintLayers { object, layers, paint, .. } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                if !paint.is_empty() && paint.len() != o.mesh.vertex_count() {
                    return Err("the object's vertex count changed since this paint layer step".into());
                }
                std::mem::swap(&mut o.paint_layers, layers);
                std::mem::swap(&mut o.mesh.paint, paint);
                o.revision += 1;
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
            Edit::Channels { object, state, .. } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                o.swap_channels(state)?;
                scene.active = Some(*object);
            }
            Edit::Material { object, material } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                std::mem::swap(&mut o.material, material);
            }
            Edit::LayerState { object, layers, positions, paint, mask, .. } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                if positions.len() != o.mesh.vertex_count() || mask.len() != o.mask.len() {
                    return Err("the object's vertex count changed since this layer step".into());
                }
                std::mem::swap(&mut o.layers, layers);
                std::mem::swap(&mut o.mesh.positions, positions);
                std::mem::swap(&mut o.mesh.paint, paint);
                std::mem::swap(&mut o.mask, mask);
                o.mesh.recompute_normals();
                o.bvh.refit(&o.mesh);
                o.revision += 1;
                scene.active = Some(*object);
            }
            Edit::LayerStroke { object, layer, edit, .. } => {
                let o = scene.get_mut(*object).ok_or("the edited object no longer exists")?;
                edit.apply(o, forward);
                edit.record_into_layer(o, *layer, if forward { 1.0 } else { -1.0 });
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

#[derive(Debug)]
pub struct History {
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// The camera the next steps record, as the window last sent it (set_history_camera); None records none.
    pub camera: Option<StepCamera>,
    max_steps: usize,
    /// Memory the steps may keep together, in bytes; 0 is no limit.
    max_bytes: usize,
}

impl Default for History {
    fn default() -> Self {
        Self { undo: Vec::new(), redo: Vec::new(), camera: None, max_steps: DEFAULT_STEPS, max_bytes: 0 }
    }
}

impl History {
    pub fn push(&mut self, edit: Edit) {
        self.redo.clear();
        self.undo.push(Step { edit, camera: self.camera });
        self.trim();
    }

    /// Forgets every step (a project opened) but keeps the limits and the camera being recorded.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Drops the oldest done steps until the count and the memory are within the limits, always keeping the newest
    /// done step (one step larger than the memory limit is still undoable). Returns how many were dropped.
    fn trim(&mut self) -> usize {
        let mut drop = self.undo.len().saturating_sub(self.max_steps);
        if self.max_bytes > 0 {
            let mut total: usize = self.undo.iter().chain(&self.redo).map(|s| s.edit.bytes()).sum::<usize>()
                - self.undo[..drop].iter().map(|s| s.edit.bytes()).sum::<usize>();
            while total > self.max_bytes && drop + 1 < self.undo.len() {
                total -= self.undo[drop].edit.bytes();
                drop += 1;
            }
        }
        self.undo.drain(..drop);
        drop
    }

    /// Sets how many steps and how much memory (bytes, 0 for no limit) the history keeps, dropping the oldest steps
    /// at once if it is over. Returns how many were dropped.
    pub fn set_limits(&mut self, max_steps: usize, max_bytes: usize) -> usize {
        self.max_steps = max_steps.clamp(1, MAX_STEPS);
        self.max_bytes = max_bytes;
        self.trim()
    }

    pub fn limits(&self) -> (usize, usize) {
        (self.max_steps, self.max_bytes)
    }

    pub fn undo(&mut self, scene: &mut Scene) -> Result<Option<String>, String> {
        let Some(mut step) = self.undo.pop() else { return Ok(None) };
        let label = step.edit.label();
        step.edit.apply(scene, false)?;
        self.redo.push(step);
        Ok(Some(label))
    }

    pub fn redo(&mut self, scene: &mut Scene) -> Result<Option<String>, String> {
        let Some(mut step) = self.redo.pop() else { return Ok(None) };
        let label = step.edit.label();
        step.edit.apply(scene, true)?;
        self.undo.push(step);
        Ok(Some(label))
    }

    pub fn depth(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }

    fn ordered(&self) -> impl Iterator<Item = &Step> {
        self.undo.iter().chain(self.redo.iter().rev())
    }

    /// Every step oldest first, and how many of them are done (the rest can be redone).
    pub fn steps(&self) -> (Vec<String>, usize) {
        (self.ordered().map(|s| s.edit.label()).collect(), self.undo.len())
    }

    /// The memory each step keeps, oldest first, in bytes.
    pub fn sizes(&self) -> Vec<usize> {
        self.ordered().map(|s| s.edit.bytes()).collect()
    }

    /// The camera each step was made from, oldest first (None where none was recorded).
    pub fn cameras(&self) -> Vec<Option<StepCamera>> {
        self.ordered().map(|s| s.camera).collect()
    }

    /// Undoes or redoes until exactly `done` steps are done.
    pub fn go_to(&mut self, scene: &mut Scene, done: usize) -> Result<usize, String> {
        if done > self.undo.len() + self.redo.len() {
            return Err(format!("there are only {} steps", self.undo.len() + self.redo.len()));
        }
        let mut moved = 0;
        while self.undo.len() > done {
            self.undo(scene)?;
            moved += 1;
        }
        while self.undo.len() < done {
            self.redo(scene)?;
            moved += 1;
        }
        Ok(moved)
    }
}
