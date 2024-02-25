use crate::render::Renderer;
use crate::ui::UiPlatform;
use bytemuck::NoUninit;
use std::ops::Range;
use std::ops::{Deref, DerefMut, RangeBounds};
use wgpu::util::DeviceExt;
use wgpu::RenderPass;

pub mod builder;
pub use builder::{InstanceBufferBuilder, VertexBufferBuilder};

pub static PRIMITIVE_POINTS: wgpu::PrimitiveState = wgpu::PrimitiveState {
    topology: wgpu::PrimitiveTopology::PointList,
    strip_index_format: None,
    front_face: wgpu::FrontFace::Ccw,
    cull_mode: None,
    polygon_mode: wgpu::PolygonMode::Point,
    unclipped_depth: false,
    conservative: false,
};

/// For instancing, use InstanceBuffer
pub struct VertexBuffer<T: NoUninit> {
    buffer: wgpu::Buffer,
    array_stride: wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode,
    attributes: Vec<wgpu::VertexAttribute>,
    vertices: Vec<T>,
}

pub struct VertexBufferGuard<'a, T: NoUninit, P: UiPlatform> {
    vertex_buffer: &'a mut VertexBuffer<T>,
    renderer: &'a Renderer<P>,
    changed: bool,
}

pub struct IndexBuffer<T: num_traits::PrimInt + NoUninit> {
    buffer: wgpu::Buffer,
    indices: Vec<T>,
    format: wgpu::IndexFormat,
}

pub struct IndexBufferGuard<'a, T: num_traits::PrimInt + NoUninit, P: UiPlatform> {
    index_buffer: &'a mut IndexBuffer<T>,
    renderer: &'a Renderer<P>,
    changed: bool,
}

/// Handles the instance and vertex buffers together
pub struct InstanceBuffer<V: NoUninit, I: NoUninit> {
    vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    vertex_array_stride: wgpu::BufferAddress,
    instance_array_stride: wgpu::BufferAddress,
    vertex_step_mode: wgpu::VertexStepMode,
    instance_step_mode: wgpu::VertexStepMode,
    vertex_attributes: Vec<wgpu::VertexAttribute>,
    instance_attributes: Vec<wgpu::VertexAttribute>,
    vertices: Vec<V>,
    instances: Vec<I>,
}

pub struct InstanceBufferGuard<'a, V: NoUninit, I: NoUninit, P: UiPlatform> {
    instance_buffer: &'a mut InstanceBuffer<V, I>,
    renderer: &'a Renderer<P>,
    changed: bool,
}

impl<T: num_traits::PrimInt + NoUninit> IndexBuffer<T> {
    pub fn new<P: UiPlatform>(
        renderer: &Renderer<P>,
        indices: &[T],
        format: wgpu::IndexFormat,
        label: Option<&str>,
    ) -> Self {
        Self::with_vec(renderer, indices.to_owned(), format, label)
    }

    pub fn with_vec<P: UiPlatform>(
        renderer: &Renderer<P>,
        indices: Vec<T>,
        format: wgpu::IndexFormat,
        label: Option<&str>,
    ) -> Self {
        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label,
                contents: bytemuck::cast_slice(indices.as_slice()),
                usage: wgpu::BufferUsages::INDEX,
            });

        Self {
            buffer,
            indices,
            format,
        }
    }

    pub fn modify<'a, P: UiPlatform>(
        &'a mut self,
        renderer: &'a Renderer<P>,
    ) -> IndexBufferGuard<'a, T, P> {
        IndexBufferGuard {
            index_buffer: self,
            renderer,
            changed: false,
        }
    }

    pub fn format(&self) -> wgpu::IndexFormat {
        self.format
    }

    pub fn as_slice(&self) -> &[T] {
        self.indices.as_slice()
    }

    pub fn as_index_buffer(&self) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(..)
    }

    pub fn slice_buffer<S: RangeBounds<wgpu::BufferAddress>>(
        &self,
        bounds: S,
    ) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(bounds)
    }

    pub fn range(&self) -> Range<u32> {
        0..self.indices.len() as u32
    }

    pub fn bind<'a, 'b: 'a>(&'b self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_index_buffer(self.as_index_buffer(), self.format);
    }
}

impl<T: NoUninit + num_traits::PrimInt> AsRef<[T]> for IndexBuffer<T> {
    fn as_ref(&self) -> &[T] {
        self.indices.as_slice()
    }
}

impl<'a, T: NoUninit + num_traits::PrimInt, P: UiPlatform> AsRef<[T]>
    for IndexBufferGuard<'a, T, P>
{
    fn as_ref(&self) -> &[T] {
        self.index_buffer.as_ref()
    }
}

impl<'a, T: NoUninit + num_traits::PrimInt, P: UiPlatform> Deref for IndexBufferGuard<'a, T, P> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.index_buffer.indices
    }
}

/// Using this will make the data be sent to the GPU on drop
impl<'a, T: NoUninit + num_traits::PrimInt, P: UiPlatform> AsMut<[T]>
    for IndexBufferGuard<'a, T, P>
{
    fn as_mut(&mut self) -> &mut [T] {
        self.changed = true;
        self.index_buffer.indices.as_mut_slice()
    }
}

/// Using this will make the data be sent to the GPU on drop
impl<'a, T: NoUninit + num_traits::PrimInt, P: UiPlatform> DerefMut for IndexBufferGuard<'a, T, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;
        &mut self.index_buffer.indices
    }
}

impl<'a, T: NoUninit + num_traits::PrimInt, P: UiPlatform> Drop for IndexBufferGuard<'a, T, P> {
    fn drop(&mut self) {
        if self.changed {
            self.renderer.queue.write_buffer(
                &self.index_buffer.buffer,
                0,
                bytemuck::cast_slice(&self.index_buffer.indices),
            );
        }
    }
}

impl<T: NoUninit> VertexBuffer<T> {
    /// Create a guard to modify the VertexBuffer
    /// When the guard drops, it wil buffer the data to the GPU
    pub fn modify<'a, P: UiPlatform>(
        &'a mut self,
        renderer: &'a Renderer<P>,
    ) -> VertexBufferGuard<'a, T, P> {
        VertexBufferGuard {
            vertex_buffer: self,
            renderer,
            changed: false,
        }
    }

    pub fn layout(&self) -> wgpu::VertexBufferLayout<'_> {
        wgpu::VertexBufferLayout {
            array_stride: self.array_stride,
            step_mode: self.step_mode,
            attributes: self.attributes.as_slice(),
        }
    }

    pub fn as_slice(&self) -> &[T] {
        self.vertices.as_slice()
    }

    pub fn as_vertex_buffer(&self) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(..)
    }

    pub fn slice_buffer<S: RangeBounds<wgpu::BufferAddress>>(
        &self,
        bounds: S,
    ) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(bounds)
    }

    pub fn range(&self) -> Range<u32> {
        0..self.vertices.len() as u32
    }

    pub fn bind<'a, 'b: 'a>(&'b self, render_pass: &mut RenderPass<'a>, slot: u32) {
        render_pass.set_vertex_buffer(slot, self.as_vertex_buffer());
    }
}

impl<T: NoUninit> AsRef<[T]> for VertexBuffer<T> {
    fn as_ref(&self) -> &[T] {
        self.vertices.as_slice()
    }
}

impl<'a, T: NoUninit, P: UiPlatform> AsRef<[T]> for VertexBufferGuard<'a, T, P> {
    fn as_ref(&self) -> &[T] {
        self.vertex_buffer.as_ref()
    }
}

impl<'a, T: NoUninit, P: UiPlatform> Deref for VertexBufferGuard<'a, T, P> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.vertex_buffer.vertices
    }
}

/// Using this will make the data be sent to the GPU on drop
impl<'a, T: NoUninit, P: UiPlatform> AsMut<[T]> for VertexBufferGuard<'a, T, P> {
    fn as_mut(&mut self) -> &mut [T] {
        self.changed = true;
        self.vertex_buffer.vertices.as_mut_slice()
    }
}

/// Using this will make the data be sent to the GPU on drop
impl<'a, T: NoUninit, P: UiPlatform> DerefMut for VertexBufferGuard<'a, T, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;
        &mut self.vertex_buffer.vertices
    }
}

impl<'a, T: NoUninit, P: UiPlatform> Drop for VertexBufferGuard<'a, T, P> {
    fn drop(&mut self) {
        if self.changed {
            self.renderer.queue.write_buffer(
                &self.vertex_buffer.buffer,
                0,
                bytemuck::cast_slice(&self.vertex_buffer.vertices),
            );
        }
    }
}

impl<V: NoUninit, I: NoUninit> InstanceBuffer<V, I> {
    /// Create a guard to modify the InstanceBuffer
    /// When the guard drops, it wil buffer the data to the GPU
    pub fn modify<'a, P: UiPlatform>(
        &'a mut self,
        renderer: &'a Renderer<P>,
    ) -> InstanceBufferGuard<'a, V, I, P> {
        InstanceBufferGuard {
            instance_buffer: self,
            renderer,
            changed: false,
        }
    }

    pub fn vertex_layout(&self) -> wgpu::VertexBufferLayout<'_> {
        wgpu::VertexBufferLayout {
            array_stride: self.vertex_array_stride,
            step_mode: self.vertex_step_mode,
            attributes: self.vertex_attributes.as_slice(),
        }
    }

    pub fn as_vertex_slice(&self) -> &[V] {
        self.vertices.as_slice()
    }

    pub fn as_vertex_buffer(&self) -> wgpu::BufferSlice<'_> {
        self.vertex_buffer.slice(..)
    }

    pub fn slice_vertex_buffer<S: RangeBounds<wgpu::BufferAddress>>(
        &self,
        bounds: S,
    ) -> wgpu::BufferSlice<'_> {
        self.vertex_buffer.slice(bounds)
    }

    pub fn range_vertex(&self) -> Range<u32> {
        0..self.vertices.len() as u32
    }

    pub fn instance_layout(&self) -> wgpu::VertexBufferLayout<'_> {
        wgpu::VertexBufferLayout {
            array_stride: self.instance_array_stride,
            step_mode: self.instance_step_mode,
            attributes: self.instance_attributes.as_slice(),
        }
    }

    pub fn as_instance_slice(&self) -> &[I] {
        self.instances.as_slice()
    }

    pub fn as_instance_buffer(&self) -> wgpu::BufferSlice<'_> {
        self.instance_buffer.slice(..)
    }

    pub fn slice_instance_buffer<S: RangeBounds<wgpu::BufferAddress>>(
        &self,
        bounds: S,
    ) -> wgpu::BufferSlice<'_> {
        self.instance_buffer.slice(bounds)
    }

    pub fn range_instance(&self) -> Range<u32> {
        0..self.instances.len() as u32
    }

    pub fn bind<'a, 'b: 'a>(&'b self, render_pass: &mut RenderPass<'a>, v_slot: u32, i_slot: u32) {
        render_pass.set_vertex_buffer(v_slot, self.as_vertex_buffer());
        render_pass.set_vertex_buffer(i_slot, self.as_instance_buffer());
    }
}

impl<'a, V: NoUninit, I: NoUninit, P: UiPlatform> Drop for InstanceBufferGuard<'a, V, I, P> {
    fn drop(&mut self) {
        if self.changed {
            self.renderer.queue.write_buffer(
                &self.instance_buffer.vertex_buffer,
                0,
                bytemuck::cast_slice(&self.instance_buffer.vertices),
            );
            self.renderer.queue.write_buffer(
                &self.instance_buffer.instance_buffer,
                0,
                bytemuck::cast_slice(&self.instance_buffer.instances),
            );
        }
    }
}
