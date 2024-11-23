use super::{RenderPipeline, Renderer, Shader};
use crate::{ui::UiPlatform, GraphicsInitError};
use std::{marker::PhantomData, num::NonZeroU32};

mod sealed {
    pub trait Sealed {}
}

pub trait BuilderState: sealed::Sealed {}

impl<T: sealed::Sealed> BuilderState for T {}

// Builder struct for a wgpu Shader
// By default, the vertex shader entry point is "vs_main"
// and there is no fragment shader entry point.
// Must add a shader module descriptor to build.
pub struct ShaderBuilder<'a, S: BuilderState> {
    module: Option<wgpu::ShaderModuleDescriptor<'a>>,
    vs_entry: &'a str,
    fs_entry: Option<&'a str>,
    buffers: Vec<wgpu::VertexBufferLayout<'a>>,
    targets: Vec<Option<wgpu::ColorTargetState>>,
    state: PhantomData<S>,
}

// Builder for a BindGroupLayout
pub struct BindGroupLayoutBuilder<'a> {
    label: Option<&'a str>,
    entries: Vec<wgpu::BindGroupLayoutEntry>,
}

// Builder struct for a rendering pipeline
// Requires adding a vertex shader (as a Shader struct)
pub struct RenderPipelineBuilder<'a, S: BuilderState> {
    vertex_shader: Option<Shader<'a>>,
    fragment_shader: Option<Shader<'a>>,
    fragment_use_vertex_shader: bool,
    pipeline_layout_label: Option<&'a str>,
    pipeline_label: Option<&'a str>,
    bind_group_layouts: Vec<&'a wgpu::BindGroupLayout>,
    push_constant_ranges: Vec<wgpu::PushConstantRange>,
    primitive: wgpu::PrimitiveState,
    depth_stencil: Option<wgpu::DepthStencilState>,
    multisample: wgpu::MultisampleState,
    multiview: Option<NonZeroU32>,
    state: PhantomData<S>,
}

pub struct BuilderInit;
pub struct BuilderComplete;

impl sealed::Sealed for BuilderInit {}
impl sealed::Sealed for BuilderComplete {}

impl Default for ShaderBuilder<'_, BuilderInit> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ShaderBuilder<'a, BuilderInit> {
    pub fn new() -> Self {
        Self {
            module: None,
            vs_entry: "vs_main",
            fs_entry: None,
            buffers: vec![],
            targets: vec![],
            state: PhantomData,
        }
    }

    pub fn with_module(
        self,
        module: wgpu::ShaderModuleDescriptor<'a>,
    ) -> ShaderBuilder<'a, BuilderComplete> {
        ShaderBuilder {
            module: Some(module),
            vs_entry: self.vs_entry,
            fs_entry: self.fs_entry,
            buffers: self.buffers,
            targets: self.targets,
            state: PhantomData,
        }
    }
}

impl<'a> ShaderBuilder<'a, BuilderComplete> {
    /// Use a Rendererer to build the completed shader
    /// Creates a Shader struct to be passed to RendererPipeline things
    /// If a fragment shader entry point is used and no color targets are set,
    /// the builder will use a default target
    pub fn build<P: UiPlatform>(self, renderer: &Renderer<P>) -> Shader<'a> {
        let Self {
            module,
            vs_entry,
            fs_entry,
            buffers,
            mut targets,
            state: _,
        } = self;
        let module = unsafe { module.unwrap_unchecked() };

        let shader = renderer.device.create_shader_module(module);

        if fs_entry.is_some() && targets.is_empty() {
            if let Some(ref surface) = renderer.surface {
                let capabilities = surface.get_capabilities(&renderer.adapter);
                let target = wgpu::ColorTargetState {
                    format: capabilities.formats[0],
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                };
                targets.push(Some(target));
            } else {
                targets.push(Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }))
            }
        }

        Shader {
            shader,
            vs_entry,
            fs_entry,
            buffers,
            targets,
        }
    }
}

impl<'a, S: BuilderState> ShaderBuilder<'a, S> {
    pub fn with_vs_entrypoint(mut self, entrypoint: &'a str) -> Self {
        self.vs_entry = entrypoint;
        self
    }

    /// Add a default fragment shader entrypoint of "fs_main"
    /// If a fragment shader entry point is used and no color targets are set,
    /// the builder will use a default target
    pub fn with_default_fs_entrypoint(mut self) -> Self {
        self.fs_entry = Some("fs_main");
        self
    }

    /// Append a buffer to the shader VertexState
    pub fn with_buffer(mut self, buffer: wgpu::VertexBufferLayout<'a>) -> Self {
        self.buffers.push(buffer);
        self
    }

    /// Set the shader's VertexState buffers to the passed vec
    pub fn with_buffers(mut self, buffers: Vec<wgpu::VertexBufferLayout<'a>>) -> Self {
        self.buffers = buffers;
        self
    }

    /// Extends the shader VertexState's buffers with a slice
    pub fn with_buffers_slice(mut self, buffers: &[wgpu::VertexBufferLayout<'a>]) -> Self {
        self.buffers.extend_from_slice(buffers);
        self
    }

    /// Extends the shader VertexState's buffers with an iterator
    pub fn with_buffers_iter(
        mut self,
        buffers: impl IntoIterator<Item = wgpu::VertexBufferLayout<'a>>,
    ) -> Self {
        self.buffers.extend(buffers);
        self
    }

    /// Appends a target to the fragment shader color targets
    /// NOTE: This will prevent the builder from inserting a default color target!
    pub fn with_target(mut self, target: Option<wgpu::ColorTargetState>) -> Self {
        self.targets.push(target);
        self
    }

    /// Set the shader FragmentState's color targets to the passed vec
    /// NOTE: This will prevent the builder from inserting a default color target if
    ///       the targets Vec is not empty!
    pub fn with_targets(mut self, targets: Vec<Option<wgpu::ColorTargetState>>) -> Self {
        self.targets = targets;
        self
    }

    /// Extends the shader FragmentState's color targets with a slice
    /// NOTE: This will prevent the builder from inserting a default color target if
    ///       the targets slice is not empty!
    pub fn with_targets_slice(mut self, targets: &[Option<wgpu::ColorTargetState>]) -> Self {
        self.targets.extend_from_slice(targets);
        self
    }

    /// Extends the shader FragmentShate's color targets with an iterator
    /// NOTE: This will prevent the builder from inserting a default color target if
    ///       the targets slice is not empty!
    pub fn with_targets_iter(
        mut self,
        targets: impl IntoIterator<Item = Option<wgpu::ColorTargetState>>,
    ) -> Self {
        self.targets.extend(targets);
        self
    }
}

impl Default for BindGroupLayoutBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> BindGroupLayoutBuilder<'a> {
    pub fn new() -> Self {
        Self {
            label: None,
            entries: vec![],
        }
    }

    pub fn with_label(mut self, label: Option<&'a str>) -> Self {
        self.label = label;
        self
    }

    /// Appends a BindGroupLayoutEntry to the BindGroupLayout
    pub fn with_entry(mut self, entry: wgpu::BindGroupLayoutEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Set the BindGroupLayout's entries to the passed vec
    pub fn with_entries(mut self, entries: Vec<wgpu::BindGroupLayoutEntry>) -> Self {
        self.entries = entries;
        self
    }

    /// Extends the BindGroupLayout's entries with a slice
    pub fn with_entries_slice(mut self, entries: &[wgpu::BindGroupLayoutEntry]) -> Self {
        self.entries.extend_from_slice(entries);
        self
    }

    /// Extends the BindGroupLayout's entries with an iterator
    pub fn with_entries_iter(
        mut self,
        entries: impl IntoIterator<Item = wgpu::BindGroupLayoutEntry>,
    ) -> Self {
        self.entries.extend(entries);
        self
    }

    pub fn build<P: UiPlatform>(self, renderer: &Renderer<P>) -> wgpu::BindGroupLayout {
        renderer
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: self.label,
                entries: self.entries.as_slice(),
            })
    }
}

impl Default for RenderPipelineBuilder<'_, BuilderInit> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> RenderPipelineBuilder<'a, BuilderInit> {
    pub fn new() -> Self {
        Self {
            vertex_shader: None,
            fragment_shader: None,
            fragment_use_vertex_shader: false,
            pipeline_layout_label: None,
            pipeline_label: None,
            bind_group_layouts: vec![],
            push_constant_ranges: vec![],
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            state: PhantomData,
        }
    }

    /// Adds a vertex shader to the pipeline in the form of the Shader struct
    /// If the shader struct contains a fragment shader, it will also set that
    /// which will overwrite any previous fragment shader set.
    /// Either use RenderPipelineBuilder::with_vertex_shader_only or
    /// set the fragment shader with RenderPipelineBuilder::with_fragment_shader
    /// after to use a different Shader (or no Shader) for the fragment stage
    pub fn with_vertex_shader(
        self,
        shader: Shader<'a>,
    ) -> RenderPipelineBuilder<'a, BuilderComplete> {
        let Self {
            vertex_shader: _,
            fragment_shader,
            fragment_use_vertex_shader: _,
            pipeline_layout_label,
            pipeline_label,
            bind_group_layouts,
            push_constant_ranges,
            primitive,
            depth_stencil,
            multisample,
            multiview,
            state: _,
        } = self;

        let (fragment_shader, fragment_use_vertex_shader) = if shader.fs_entry.is_some() {
            (None, true)
        } else {
            (fragment_shader, false)
        };

        RenderPipelineBuilder {
            vertex_shader: Some(shader),
            fragment_shader,
            fragment_use_vertex_shader,
            pipeline_layout_label,
            pipeline_label,
            bind_group_layouts,
            push_constant_ranges,
            primitive,
            depth_stencil,
            multisample,
            multiview,
            state: PhantomData,
        }
    }

    /// Adds a vertex shader to the pipeline in the form of the Shader struct
    /// If the shader struct contains a fragment shader, it will NOT set that.
    /// Use RenderPipelineBuilder::with_vertex_shader to set both vertex and fragment shader together
    pub fn with_vertex_shader_only(
        self,
        shader: Shader<'a>,
    ) -> RenderPipelineBuilder<'a, BuilderComplete> {
        let Self {
            vertex_shader: _,
            fragment_shader,
            fragment_use_vertex_shader: _,
            pipeline_layout_label,
            pipeline_label,
            bind_group_layouts,
            push_constant_ranges,
            primitive,
            depth_stencil,
            multisample,
            multiview,
            state: _,
        } = self;

        RenderPipelineBuilder {
            vertex_shader: Some(shader),
            fragment_shader,
            fragment_use_vertex_shader: false,
            pipeline_layout_label,
            pipeline_label,
            bind_group_layouts,
            push_constant_ranges,
            primitive,
            depth_stencil,
            multisample,
            multiview,
            state: PhantomData,
        }
    }
}

impl RenderPipelineBuilder<'_, BuilderComplete> {
    /// Use a Renderer to build the completed pipeline.
    /// This pipeline is used when calling Renderer::render
    pub fn build<P: UiPlatform>(self, renderer: &Renderer<P>) -> RenderPipeline {
        let Self {
            vertex_shader,
            fragment_shader,
            fragment_use_vertex_shader,
            pipeline_layout_label,
            pipeline_label,
            bind_group_layouts,
            push_constant_ranges,
            primitive,
            depth_stencil,
            multisample,
            multiview,
            state: _,
        } = self;

        let vertex_shader = unsafe { vertex_shader.unwrap_unchecked() };

        let vertex_state = wgpu::VertexState {
            module: &vertex_shader.shader,
            entry_point: vertex_shader.vs_entry,
            buffers: vertex_shader.buffers.as_slice(),
            compilation_options: Default::default(),
        };

        let fragment_state = if fragment_use_vertex_shader {
            Some(wgpu::FragmentState {
                module: &vertex_shader.shader,
                entry_point: unsafe { vertex_shader.fs_entry.unwrap_unchecked() },
                targets: vertex_shader.targets.as_slice(),
                compilation_options: Default::default(),
            })
        } else {
            fragment_shader.as_ref().map(|shader| wgpu::FragmentState {
                module: &shader.shader,
                entry_point: unsafe { shader.fs_entry.unwrap_unchecked() },
                targets: shader.targets.as_slice(),
                compilation_options: Default::default(),
            })
        };

        let layout = renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: pipeline_layout_label,
                bind_group_layouts: bind_group_layouts.as_slice(),
                push_constant_ranges: push_constant_ranges.as_slice(),
            });

        let pipeline = renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: pipeline_label,
                layout: Some(&layout),
                vertex: vertex_state,
                fragment: fragment_state,
                primitive,
                depth_stencil,
                multisample,
                multiview,
                cache: None,
            });

        RenderPipeline { layout, pipeline }
    }
}

impl<'a, S: BuilderState> RenderPipelineBuilder<'a, S> {
    /// Sets the fragment shader. Will override any current fragment shader set.
    /// To use the fragment shader with the vertex shader, use
    /// RenderPipelineBuilder::with_vertex_shader .
    /// Returns Err(self) if shader is not None but does not have an fs_entry
    pub fn with_fragment_shader(
        mut self,
        shader: Option<Shader<'a>>,
    ) -> Result<Self, GraphicsInitError> {
        if shader
            .as_ref()
            .is_some_and(|shader| shader.fs_entry.is_none())
        {
            Err(GraphicsInitError::FailedFragmentAttach)
        } else {
            self.fragment_shader = shader;
            self.fragment_use_vertex_shader = false;
            Ok(self)
        }
    }

    /// Sets the fragment shader. Will override any current fragment shader set.
    /// To use the fragment shader with the vertex shader, use
    /// RenderPipelineBuilder::with_vertex_shader .
    /// Does not check if the given shader actually has a fragment shader entry point
    ///
    /// # Safety
    /// Only use when you're certain the given shader is None or has a non-None fs_entry
    pub unsafe fn with_fragment_shader_unchecked(mut self, shader: Option<Shader<'a>>) -> Self {
        self.fragment_shader = shader;
        self.fragment_use_vertex_shader = false;
        self
    }

    pub fn with_layout_label(mut self, label: Option<&'a str>) -> Self {
        self.pipeline_layout_label = label;
        self
    }

    pub fn with_pipeline_label(mut self, label: Option<&'a str>) -> Self {
        self.pipeline_label = label;
        self
    }

    /// Append a BindGroupLayout to the pipeline
    pub fn with_bind_group_layout(mut self, layout: &'a wgpu::BindGroupLayout) -> Self {
        self.bind_group_layouts.push(layout);
        self
    }

    /// Set the pipeline's bind_group_layouts to the passed vec
    pub fn with_bind_group_layouts(mut self, layouts: Vec<&'a wgpu::BindGroupLayout>) -> Self {
        self.bind_group_layouts = layouts;
        self
    }

    /// Append the slice of BindGroupLayout's to the pipeline's bind_group_layouts
    pub fn with_bind_group_layouts_slice(mut self, layouts: &[&'a wgpu::BindGroupLayout]) -> Self {
        self.bind_group_layouts.extend_from_slice(layouts);
        self
    }

    /// Append the iterator of BindGroupLayout's to the pipeline's bind_group_layouts
    pub fn with_bind_group_layouts_iter(
        mut self,
        layouts: impl IntoIterator<Item = &'a wgpu::BindGroupLayout>,
    ) -> Self {
        self.bind_group_layouts.extend(layouts);
        self
    }

    /// Append a PushConstantRange to the pipeline
    pub fn with_push_constant_range(mut self, constant_range: wgpu::PushConstantRange) -> Self {
        self.push_constant_ranges.push(constant_range);
        self
    }

    /// Set the pipeline's push_constant_ranges to the passed vec
    pub fn with_push_constant_ranges(
        mut self,
        constant_ranges: Vec<wgpu::PushConstantRange>,
    ) -> Self {
        self.push_constant_ranges = constant_ranges;
        self
    }

    /// Append the slice of PushConstantRange's to the pipeline's push_constant_ranges
    pub fn with_push_constant_ranges_slice(
        mut self,
        constant_ranges: &[wgpu::PushConstantRange],
    ) -> Self {
        self.push_constant_ranges.extend_from_slice(constant_ranges);
        self
    }

    /// Append the iterator of PushConstantRange's to the pipeline's push_constant_ranges
    pub fn with_push_constant_ranges_iter(
        mut self,
        constant_ranges: impl IntoIterator<Item = wgpu::PushConstantRange>,
    ) -> Self {
        self.push_constant_ranges.extend(constant_ranges);
        self
    }

    pub fn with_primitive_state(mut self, primitive: wgpu::PrimitiveState) -> Self {
        self.primitive = primitive;
        self
    }

    pub fn with_depth_stencil(mut self, depth_stencil: Option<wgpu::DepthStencilState>) -> Self {
        self.depth_stencil = depth_stencil;
        self
    }

    pub fn with_multisample(mut self, multisample: wgpu::MultisampleState) -> Self {
        self.multisample = multisample;
        self
    }

    pub fn with_multiview(mut self, multiview: Option<NonZeroU32>) -> Self {
        self.multiview = multiview;
        self
    }
}
