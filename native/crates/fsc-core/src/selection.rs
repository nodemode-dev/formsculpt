use crate::layers::Layers;
use crate::mesh::Paint;
use crate::paint_layers::PaintLayers;
use crate::scene::Object;

pub const SET_CHANNELS: [&str; 5] = ["groups", "mask", "paint", "layers", "uvs"];

#[derive(Debug, Clone, Default)]
pub struct SelectionSet {
    pub name: String,
    pub vertices: usize,
    pub faces: usize,
    pub groups: Option<Vec<u32>>,
    pub mask: Option<Vec<f32>>,
    pub paint: Option<Vec<Paint>>,
    pub strengths: Option<Vec<f32>>,
    pub uvs: Option<Vec<[f32; 2]>>,
}

impl SelectionSet {
    pub fn channels(&self) -> Vec<&'static str> {
        let present = [self.groups.is_some(), self.mask.is_some(), self.paint.is_some(), self.strengths.is_some(), self.uvs.is_some()];
        SET_CHANNELS.iter().zip(present).filter(|(_, p)| *p).map(|(c, _)| *c).collect()
    }
}

/// Channel data to swap with an object's own: whatever is Some trades places with the object's current value, so the
/// same state serves as the undo record.
#[derive(Debug, Clone, Default)]
pub struct ChannelState {
    pub groups: Option<Vec<u32>>,
    pub mask: Option<Vec<f32>>,
    pub paint: Option<(Vec<Paint>, PaintLayers)>,
    pub layers: Option<(Layers, Vec<[f32; 3]>)>,
    pub uvs: Option<Vec<[f32; 2]>>,
}

impl Object {
    pub fn swap_channels(&mut self, st: &mut ChannelState) -> Result<(), String> {
        let (v, t, c) = (self.mesh.vertex_count(), self.mesh.triangle_count(), self.mesh.indices.len());
        let fits = st.groups.as_ref().is_none_or(|g| g.is_empty() || g.len() == t)
            && st.mask.as_ref().is_none_or(|m| m.len() == v)
            && st.paint.as_ref().is_none_or(|(p, _)| p.is_empty() || p.len() == v)
            && st.layers.as_ref().is_none_or(|(l, p)| p.len() == v && l.list.iter().all(|x| x.delta.len() == v))
            && st.uvs.as_ref().is_none_or(|u| u.is_empty() || u.len() == c);
        if !fits {
            return Err("the object's vertex or face count changed since this selection step".into());
        }
        if let Some(g) = st.groups.as_mut() {
            std::mem::swap(&mut self.mesh.groups, g);
        }
        if let Some(m) = st.mask.as_mut() {
            std::mem::swap(&mut self.mask, m);
        }
        if let Some((p, l)) = st.paint.as_mut() {
            std::mem::swap(&mut self.mesh.paint, p);
            std::mem::swap(&mut self.paint_layers, l);
        }
        if let Some(u) = st.uvs.as_mut() {
            std::mem::swap(&mut self.mesh.uvs, u);
        }
        if let Some((l, p)) = st.layers.as_mut() {
            std::mem::swap(&mut self.layers, l);
            std::mem::swap(&mut self.mesh.positions, p);
            self.mesh.recompute_normals();
            self.bvh.refit(&self.mesh);
        }
        self.revision += 1;
        Ok(())
    }
}
