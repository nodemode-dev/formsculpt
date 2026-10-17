//! How much memory an undo step keeps: the arrays it holds, counted from their lengths (so weighing a step costs
//! nothing like copying it). Small fixed fields and allocator slack are left out; the figure is what the history
//! panel shows and what its memory limit is measured against.

use std::mem::size_of;

use fsc_core::{ChannelState, Layers, Maps, Mesh, Multires, Object, PaintLayers, Stencil, Texture, TextureEdit};
use fsc_sculpt::VertexEdit;

fn len<T>(v: &[T]) -> usize {
    std::mem::size_of_val(v)
}

fn opt<T>(v: &Option<Vec<T>>) -> usize {
    v.as_deref().map_or(0, len)
}

pub(crate) fn mesh(m: &Mesh) -> usize {
    len(&m.positions) + len(&m.normals) + len(&m.indices) + len(&m.paint) + len(&m.uvs) + len(&m.groups) + len(&m.hidden) + len(&m.creases)
}

pub(crate) fn stencil(s: &Stencil) -> usize {
    len(&s.offsets) + len(&s.sources) + len(&s.weights)
}

pub(crate) fn multires(m: &Option<Multires>) -> usize {
    m.as_ref().map_or(0, |m| m.levels.iter().map(|l| mesh(&l.mesh) + stencil(&l.stencil) + len(&l.evens) + len(&l.detail)).sum())
}

pub(crate) fn layers(l: &Layers) -> usize {
    l.list.iter().map(|x| x.name.len() + len(&x.delta) + opt(&x.paint) + opt(&x.mask)).sum()
}

pub(crate) fn paint_layers(p: &PaintLayers) -> usize {
    p.list.iter().map(|x| x.name.len() + len(&x.data)).sum::<usize>() + len(&p.base)
}

pub(crate) fn texture(t: &Option<Texture>) -> usize {
    t.as_ref().map_or(0, |t| len(&t.texels) + len(&t.interior))
}

fn maps(m: &Maps) -> usize {
    m.color_png.len() + m.mr_png.len() + m.normal_png.len() + m.displacement_exr.len() + m.emissive_png.len()
}

/// A whole object as a step parks it (deleted, replaced, cleared): its meshes, levels, layers, paint, texture and
/// maps, and the search tree and adjacency it carries (the tree's private parent links counted as one index per node
/// and per triangle).
pub(crate) fn object(o: &Object) -> usize {
    let bvh = len(&o.bvh.nodes) + len(&o.bvh.tris) + (o.bvh.nodes.len() + o.bvh.tris.len()) * size_of::<u32>();
    let topology = len(&o.topology.face_offsets) + len(&o.topology.faces) + len(&o.topology.neighbor_offsets) + len(&o.topology.neighbors);
    o.name.len()
        + mesh(&o.mesh)
        + bvh
        + topology
        + len(&o.mask)
        + multires(&o.multires)
        + layers(&o.layers)
        + o.maps.as_ref().map_or(0, maps)
        + texture(&o.texture)
        + o.tile_maps.iter().map(|(_, m)| maps(m)).sum::<usize>()
        + paint_layers(&o.paint_layers)
}

pub(crate) fn vertex_edit(e: &VertexEdit) -> usize {
    len(&e.indices)
        + len(&e.before)
        + len(&e.after)
        + len(&e.mask_before)
        + len(&e.mask_after)
        + len(&e.paint_before)
        + len(&e.paint_after)
        + e.layer.as_ref().map_or(0, |(_, a, b)| len(a) + len(b))
}

pub(crate) fn texture_edit(e: &TextureEdit) -> usize {
    len(&e.texels) + len(&e.before) + len(&e.after)
}

pub(crate) fn channels(c: &ChannelState) -> usize {
    opt(&c.groups)
        + opt(&c.mask)
        + c.paint.as_ref().map_or(0, |(p, l)| len(p) + paint_layers(l))
        + c.layers.as_ref().map_or(0, |(l, p)| layers(l) + len(p))
        + opt(&c.uvs)
}
