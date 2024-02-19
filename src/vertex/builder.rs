use super::VertexBuffer;
use crate::{render::Renderer, ui::UiPlatform};
use bytemuck::NoUninit;
use wgpu::util::DeviceExt;

/// Builder struct for a wgpu VertexBuffer
/// Creates the VertexBufferLayout as well
/// The default array_stride of the layout is the memory size
/// of T. The default step_mode of the layout is wgpu::VertexStepMode::Vertex.
pub struct VertexBufferBuilder<'a, T: NoUninit> {
    attributes: Vec<wgpu::VertexAttribute>,
    array_stride: wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode,
    label: Option<&'a str>,
    data: Vec<T>,
}

impl<'a, T: NoUninit> Default for VertexBufferBuilder<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T: NoUninit> VertexBufferBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            attributes: vec![],
            array_stride: std::mem::size_of::<T>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            label: None,
            data: vec![],
        }
    }

    /// Creates the VertexBuffer
    /// This includes calls to the GPU
    pub fn build<P: UiPlatform>(self, renderer: &Renderer<P>) -> VertexBuffer<T> {
        let Self {
            attributes,
            array_stride,
            step_mode,
            label,
            data,
        } = self;

        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label,
                contents: bytemuck::cast_slice(data.as_slice()),
                usage: wgpu::BufferUsages::VERTEX,
            });

        VertexBuffer {
            buffer,
            array_stride,
            step_mode,
            attributes,
            vertices: data,
        }
    }

    /// Sets the initial vertices of the buffer.
    /// Will override any previously set vertices.
    pub fn with_initial_vertices(mut self, initial_vertices: &[T]) -> Self {
        self.data.clear();
        self.data.extend_from_slice(initial_vertices);
        self
    }

    /// Sets the initial vertices of the buffer.
    /// Will override any previously set vertices.
    pub fn with_initial_vertices_owned(mut self, initial_vertices: Vec<T>) -> Self {
        self.data = initial_vertices;
        self
    }

    /// Extends the current initial vertices of the buffer with a slice
    pub fn extend_initial_vertices_from_slice(mut self, extra_vertices: &[T]) -> Self {
        self.data.extend_from_slice(extra_vertices);
        self
    }

    /// Extends current initial vertices of the buffer with an iterator
    pub fn extend_initial_vertices(mut self, extra_vertices: impl IntoIterator<Item = T>) -> Self {
        self.data.extend(extra_vertices);
        self
    }

    /// Sets the array_stride of the VertexBufferLayout, overriding any previous value
    pub fn with_array_stride(mut self, stride: wgpu::BufferAddress) -> Self {
        self.array_stride = stride;
        self
    }

    /// Sets the step_mode of the VertexBufferLayout, overriding any previous value
    pub fn with_step_mode(mut self, step_mode: wgpu::VertexStepMode) -> Self {
        self.step_mode = step_mode;
        self
    }

    /// Sets the label of the VertexBufferLayout and the VertexBuffer, overriding any previous value
    pub fn with_label(mut self, label: Option<&'a str>) -> Self {
        self.label = label;
        self
    }

    /// Sets the VertexAttribute's of the layout to the slice, overriding any previous attributes
    pub fn with_attributes(mut self, attributes: &[wgpu::VertexAttribute]) -> Self {
        self.attributes.clear();
        self.attributes.extend_from_slice(attributes);
        self
    }

    /// Sets the VertexAttribute's of the layout, overriding any previous attributes
    pub fn with_attributes_owned(mut self, attributes: Vec<wgpu::VertexAttribute>) -> Self {
        self.attributes = attributes;
        self
    }

    /// Extends the current vertex attributes with a slice
    pub fn extend_attributes_from_slice(mut self, attributes: &[wgpu::VertexAttribute]) -> Self {
        self.attributes.extend_from_slice(attributes);
        self
    }

    /// Extends the current vertex attributes with an iterator
    pub fn extend_attributes(
        mut self,
        attributes: impl IntoIterator<Item = wgpu::VertexAttribute>,
    ) -> Self {
        self.attributes.extend(attributes);
        self
    }
}
