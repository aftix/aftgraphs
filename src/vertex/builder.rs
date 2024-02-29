use super::{InstanceBuffer, VertexBuffer};
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
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        VertexBuffer {
            buffer,
            array_stride,
            step_mode,
            attributes,
            vertices: data,
            label: label.map(String::from),
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

/// Builder struct for a wgpu InstanceBuffer
/// Creates the VertexBufferLayout's as well
/// The default *_array_stride of the layout is the memory size
/// of {V,I}. The default *_step_mode of the layout is wgpu::VertexStepMode::Vertex.
pub struct InstanceBufferBuilder<'a, V: NoUninit, I: NoUninit> {
    vertex_attributes: Vec<wgpu::VertexAttribute>,
    instance_attributes: Vec<wgpu::VertexAttribute>,
    vertex_array_stride: wgpu::BufferAddress,
    instance_array_stride: wgpu::BufferAddress,
    vertex_step_mode: wgpu::VertexStepMode,
    instance_step_mode: wgpu::VertexStepMode,
    v_label: Option<&'a str>,
    i_label: Option<&'a str>,
    v_data: Vec<V>,
    i_data: Vec<I>,
}

impl<'a, V: NoUninit, I: NoUninit> Default for InstanceBufferBuilder<'a, V, I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, V: NoUninit, I: NoUninit> InstanceBufferBuilder<'a, V, I> {
    pub fn new() -> Self {
        Self {
            vertex_attributes: vec![],
            instance_attributes: vec![],
            vertex_array_stride: std::mem::size_of::<V>() as wgpu::BufferAddress,
            instance_array_stride: std::mem::size_of::<I>() as wgpu::BufferAddress,
            vertex_step_mode: wgpu::VertexStepMode::Vertex,
            instance_step_mode: wgpu::VertexStepMode::Instance,
            v_label: None,
            i_label: None,
            v_data: vec![],
            i_data: vec![],
        }
    }

    /// Creates the InstanceBuffer
    /// This includes calls to the GPU
    pub fn build<P: UiPlatform>(self, renderer: &Renderer<P>) -> InstanceBuffer<V, I> {
        let Self {
            vertex_attributes,
            instance_attributes,
            vertex_array_stride,
            instance_array_stride,
            vertex_step_mode,
            instance_step_mode,
            v_label,
            i_label,
            v_data,
            i_data,
        } = self;

        let vertex_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: v_label,
                contents: bytemuck::cast_slice(v_data.as_slice()),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let instance_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: i_label,
                    contents: bytemuck::cast_slice(i_data.as_slice()),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });

        InstanceBuffer {
            vertex_buffer,
            instance_buffer,
            vertex_array_stride,
            instance_array_stride,
            vertex_step_mode,
            instance_step_mode,
            vertex_attributes,
            instance_attributes,
            vertices: v_data,
            instances: i_data,
            instance_label: i_label.map(String::from),
            vertex_label: v_label.map(String::from),
        }
    }

    /// Sets the initial vertices of the buffer.
    /// Will override any previously set vertices.
    pub fn with_initial_vertices(mut self, initial_vertices: &[V]) -> Self {
        self.v_data.clear();
        self.v_data.extend_from_slice(initial_vertices);
        self
    }

    /// Sets the initial instances of the buffer.
    /// Will override any previously set instances.
    pub fn with_initial_instances(mut self, initial_instances: &[I]) -> Self {
        self.i_data.clear();
        self.i_data.extend_from_slice(initial_instances);
        self
    }

    /// Sets the initial vertices of the buffer.
    /// Will override any previously set vertices.
    pub fn with_initial_vertices_owned(mut self, initial_vertices: Vec<V>) -> Self {
        self.v_data = initial_vertices;
        self
    }

    /// Sets the initial instances of the buffer.
    /// Will override any previously set instances.
    pub fn with_initial_instances_owned(mut self, initial_instances: Vec<I>) -> Self {
        self.i_data = initial_instances;
        self
    }

    /// Extends the current initial vertices of the buffer with a slice
    pub fn extend_initial_vertices_from_slice(mut self, extra_vertices: &[V]) -> Self {
        self.v_data.extend_from_slice(extra_vertices);
        self
    }

    /// Extends the current initial vertices of the buffer with a slice
    pub fn extend_initial_instances_from_slice(mut self, extra_vertices: &[I]) -> Self {
        self.i_data.extend_from_slice(extra_vertices);
        self
    }

    /// Extends current initial vertices of the buffer with an iterator
    pub fn extend_initial_vertices(mut self, extra_vertices: impl IntoIterator<Item = V>) -> Self {
        self.v_data.extend(extra_vertices);
        self
    }

    /// Extends current initial vertices of the buffer with an iterator
    pub fn extend_initial_instances(
        mut self,
        extra_instances: impl IntoIterator<Item = I>,
    ) -> Self {
        self.i_data.extend(extra_instances);
        self
    }

    /// Sets the array_stride of the vertices' VertexBufferLayout, overriding any previous value
    pub fn with_vertex_array_stride(mut self, stride: wgpu::BufferAddress) -> Self {
        self.vertex_array_stride = stride;
        self
    }

    /// Sets the array_stride of the instances' VertexBufferLayout, overriding any previous value
    pub fn with_instance_array_stride(mut self, stride: wgpu::BufferAddress) -> Self {
        self.instance_array_stride = stride;
        self
    }

    /// Sets the step_mode of the vertices' VertexBufferLayout, overriding any previous value
    pub fn with_vertex_step_mode(mut self, step_mode: wgpu::VertexStepMode) -> Self {
        self.vertex_step_mode = step_mode;
        self
    }

    /// Sets the step_mode of the instances' VertexBufferLayout, overriding any previous value
    pub fn with_instance_step_mode(mut self, step_mode: wgpu::VertexStepMode) -> Self {
        self.instance_step_mode = step_mode;
        self
    }

    /// Sets the label of the vertices' VertexBufferLayout and the VertexBuffer, overriding any previous value
    pub fn with_vertex_label(mut self, label: Option<&'a str>) -> Self {
        self.v_label = label;
        self
    }

    /// Sets the label of the instances' VertexBufferLayout and the VertexBuffer, overriding any previous value
    pub fn with_instance_label(mut self, label: Option<&'a str>) -> Self {
        self.i_label = label;
        self
    }

    /// Sets the VertexAttribute's of the vertices' layout to the slice, overriding any previous attributes
    pub fn with_vertex_attributes(mut self, attributes: &[wgpu::VertexAttribute]) -> Self {
        self.vertex_attributes.clear();
        self.vertex_attributes.extend_from_slice(attributes);
        self
    }

    /// Sets the VertexAttribute's of the layout to the slice, overriding any previous attributes
    pub fn with_instance_attributes(mut self, attributes: &[wgpu::VertexAttribute]) -> Self {
        self.instance_attributes.clear();
        self.instance_attributes.extend_from_slice(attributes);
        self
    }

    /// Sets the vertices' VertexAttribute's of the layout, overriding any previous attributes
    pub fn with_vertex_attributes_owned(mut self, attributes: Vec<wgpu::VertexAttribute>) -> Self {
        self.vertex_attributes = attributes;
        self
    }

    /// Sets the instances' VertexAttribute's of the layout, overriding any previous attributes
    pub fn with_instance_attributes_owned(
        mut self,
        attributes: Vec<wgpu::VertexAttribute>,
    ) -> Self {
        self.instance_attributes = attributes;
        self
    }

    /// Extends the current vertices' vertex attributes with a slice
    pub fn extend_vertex_attributes_from_slice(
        mut self,
        attributes: &[wgpu::VertexAttribute],
    ) -> Self {
        self.vertex_attributes.extend_from_slice(attributes);
        self
    }

    /// Extends the current instances' vertex attributes with a slice
    pub fn extend_instance_attributes_from_slice(
        mut self,
        attributes: &[wgpu::VertexAttribute],
    ) -> Self {
        self.instance_attributes.extend_from_slice(attributes);
        self
    }

    /// Extends the current vertices' vertex attributes with an iterator
    pub fn extend_vertex_attributes(
        mut self,
        attributes: impl IntoIterator<Item = wgpu::VertexAttribute>,
    ) -> Self {
        self.vertex_attributes.extend(attributes);
        self
    }

    /// Extends the current instances' vertex attributes with an iterator
    pub fn extend_instance_attributes(
        mut self,
        attributes: impl IntoIterator<Item = wgpu::VertexAttribute>,
    ) -> Self {
        self.instance_attributes.extend(attributes);
        self
    }
}
