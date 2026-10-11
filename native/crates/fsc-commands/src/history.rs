use fsc_core::{Object, ObjectId, Scene};
use fsc_sculpt::VertexEdit;

const LIMIT: usize = 200;

#[derive(Debug)]
pub enum Edit {
    Vertices { label: String, object: ObjectId, edit: VertexEdit },
    Add { id: ObjectId, index: usize, parked: Option<Object> },
    Delete { id: ObjectId, index: usize, parked: Option<Object> },
    Clear { parked: Vec<Object>, active: Option<ObjectId> },
}

impl Edit {
    pub fn label(&self) -> String {
        match self {
            Edit::Vertices { label, .. } => label.clone(),
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
            Edit::Add { id, index, parked } | Edit::Delete { id, index, parked } => {
                if let Some(o) = parked.take() {
                    scene.insert(*index, o);
                } else {
                    *index = scene.index_of(*id).ok_or("the object is already gone")?;
                    *parked = scene.remove(*id);
                }
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
