use crate::render::Renderer;
use bytemuck::NoUninit;
use std::ops::Range;
use std::ops::{Deref, DerefMut, RangeBounds};
use wgpu::util::DeviceExt;

pub mod builder;
pub use builder::VertexBufferBuilder;

pub static PRIMITIVE_POINTS: wgpu::PrimitiveState = wgpu::PrimitiveState {
    topology: wgpu::PrimitiveTopology::PointList,
    strip_index_format: None,
    front_face: wgpu::FrontFace::Ccw,
    cull_mode: None,
    polygon_mode: wgpu::PolygonMode::Point,
    unclipped_depth: false,
    conservative: false,
};

pub struct VertexBuffer<T: NoUninit> {
    buffer: wgpu::Buffer,
    array_stride: wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode,
    attributes: Vec<wgpu::VertexAttribute>,
    vertices: Vec<T>,
}

pub struct VertexBufferGuard<'a, T: NoUninit> {
    vertex_buffer: &'a mut VertexBuffer<T>,
    renderer: &'a Renderer,
    changed: bool,
}

pub struct IndexBuffer<T: num_traits::PrimInt + NoUninit> {
    buffer: wgpu::Buffer,
    indices: Vec<T>,
    format: wgpu::IndexFormat,
}

pub struct IndexBufferGuard<'a, T: num_traits::PrimInt + NoUninit> {
    index_buffer: &'a mut IndexBuffer<T>,
    renderer: &'a Renderer,
    changed: bool,
}

impl<T: num_traits::PrimInt + NoUninit> IndexBuffer<T> {
    pub fn new(
        renderer: &Renderer,
        indices: &[T],
        format: wgpu::IndexFormat,
        label: Option<&str>,
    ) -> Self {
        Self::with_vec(renderer, indices.to_owned(), format, label)
    }

    pub fn with_vec(
        renderer: &Renderer,
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

    pub fn modify<'a>(&'a mut self, renderer: &'a Renderer) -> IndexBufferGuard<'a, T> {
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
}

impl<T: NoUninit + num_traits::PrimInt> AsRef<[T]> for IndexBuffer<T> {
    fn as_ref(&self) -> &[T] {
        self.indices.as_slice()
    }
}

impl<'a, T: NoUninit + num_traits::PrimInt> AsRef<[T]> for IndexBufferGuard<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.index_buffer.as_ref()
    }
}

impl<'a, T: NoUninit + num_traits::PrimInt> Deref for IndexBufferGuard<'a, T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.index_buffer.indices
    }
}

/// Using this will make the data be sent to the GPU on drop
impl<'a, T: NoUninit + num_traits::PrimInt> AsMut<[T]> for IndexBufferGuard<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.changed = true;
        self.index_buffer.indices.as_mut_slice()
    }
}

/// Using this will make the data be sent to the GPU on drop
impl<'a, T: NoUninit + num_traits::PrimInt> DerefMut for IndexBufferGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;
        &mut self.index_buffer.indices
    }
}

impl<'a, T: NoUninit + num_traits::PrimInt> Drop for IndexBufferGuard<'a, T> {
    fn drop(&mut self) {
        if self.changed {
            self.renderer.queue.write_buffer(
                &self.index_buffer.buffer,
                0,
                bytemuck::cast_slice(&self.index_buffer.indices),
            )
        }
    }
}

impl<T: NoUninit> VertexBuffer<T> {
    /// Create a guard to modify the VertexBuffer
    /// When the guard drops, it wil buffer the data to the GPU
    pub fn modify<'a>(&'a mut self, renderer: &'a Renderer) -> VertexBufferGuard<'a, T> {
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
}

impl<T: NoUninit> AsRef<[T]> for VertexBuffer<T> {
    fn as_ref(&self) -> &[T] {
        self.vertices.as_slice()
    }
}

impl<'a, T: NoUninit> AsRef<[T]> for VertexBufferGuard<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.vertex_buffer.as_ref()
    }
}

impl<'a, T: NoUninit> Deref for VertexBufferGuard<'a, T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.vertex_buffer.vertices
    }
}

/// Using this will make the data be sent to the GPU on drop
impl<'a, T: NoUninit> AsMut<[T]> for VertexBufferGuard<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.changed = true;
        self.vertex_buffer.vertices.as_mut_slice()
    }
}

/// Using this will make the data be sent to the GPU on drop
impl<'a, T: NoUninit> DerefMut for VertexBufferGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;
        &mut self.vertex_buffer.vertices
    }
}

impl<'a, T: NoUninit> Drop for VertexBufferGuard<'a, T> {
    fn drop(&mut self) {
        if self.changed {
            self.renderer.queue.write_buffer(
                &self.vertex_buffer.buffer,
                0,
                bytemuck::cast_slice(&self.vertex_buffer.vertices),
            )
        }
    }
}