use super::Uniform;
use crate::{render::Renderer, ui::UiPlatform};
use bytemuck::{NoUninit, Zeroable};
use std::marker::PhantomData;
use wgpu::util::{BufferInitDescriptor, DeviceExt};

mod sealed {
    pub trait Sealed {
        type AddBindGroupLayout: Sealed;
        type AddData: Sealed;
    }
}

pub trait BuilderState: sealed::Sealed {
    type AddBindGroupLayout: sealed::Sealed;
    type AddData: sealed::Sealed;
}

pub struct BuilderInit;
pub struct BuilderLayout;
pub struct BuilderData;
pub struct BuilderComplete;

impl sealed::Sealed for BuilderInit {
    type AddBindGroupLayout = BuilderLayout;
    type AddData = BuilderData;
}
impl sealed::Sealed for BuilderLayout {
    type AddBindGroupLayout = Self;
    type AddData = BuilderComplete;
}
impl sealed::Sealed for BuilderData {
    type AddBindGroupLayout = BuilderComplete;
    type AddData = Self;
}
impl sealed::Sealed for BuilderComplete {
    type AddBindGroupLayout = Self;
    type AddData = Self;
}

impl<T: sealed::Sealed> BuilderState for T {
    type AddBindGroupLayout = T::AddBindGroupLayout;
    type AddData = T::AddData;
}

pub struct UniformBuilder<'a, T: NoUninit, S: BuilderState> {
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    usage: wgpu::BufferUsages,
    label: Option<&'a str>,
    data: Option<T>,
    state: PhantomData<S>,
}

impl<T: NoUninit> Default for UniformBuilder<'_, T, BuilderInit> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: NoUninit> UniformBuilder<'_, T, BuilderInit> {
    pub fn new() -> Self {
        Self {
            bind_group_layout: None,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            label: None,
            data: None,
            state: PhantomData,
        }
    }
}

impl<T: NoUninit> UniformBuilder<'_, T, BuilderComplete> {
    pub fn build<P: UiPlatform>(self, renderer: &Renderer<P>) -> Uniform<T> {
        let Self {
            bind_group_layout,
            usage,
            label,
            data,
            state: _,
        } = self;

        let bind_group_layout = unsafe { bind_group_layout.unwrap_unchecked() };
        let data = unsafe { data.unwrap_unchecked() };

        let buffer = renderer.device.create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::bytes_of(&data),
            usage,
        });

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label,
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });

        Uniform {
            buffer,
            bind_group_layout,
            bind_group,
            data,
        }
    }
}

impl<'a, T: NoUninit, S: BuilderState> UniformBuilder<'a, T, S> {
    /// Add a label to the uniform
    /// The label will be applied to the bind group layout, the buffer, and the bind group
    pub fn with_label(mut self, label: Option<&'a str>) -> Self {
        self.label = label;
        self
    }

    /// Adds a BindGroupLayout to the uniform
    /// This will replace any previous layout
    /// see aftgraphs::Renderer::BindGroupLayoutBuilder
    pub fn with_bind_group_layout(
        self,
        layout: wgpu::BindGroupLayout,
    ) -> UniformBuilder<'a, T, <S as sealed::Sealed>::AddBindGroupLayout> {
        UniformBuilder {
            bind_group_layout: Some(layout),
            usage: self.usage,
            label: self.label,
            data: self.data,
            state: PhantomData,
        }
    }

    /// Adds initial data to the uniform
    /// This will reset any previous data
    /// The data is not sent to the GPU until UniformBuilder::build is called
    pub fn with_data(self, data: T) -> UniformBuilder<'a, T, <S as sealed::Sealed>::AddData> {
        UniformBuilder {
            bind_group_layout: self.bind_group_layout,
            usage: self.usage,
            label: self.label,
            data: Some(data),
            state: PhantomData,
        }
    }

    /// Sets the usage for the uniform buffer
    /// Defaults to wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
    pub fn with_buffer_usage(mut self, usage: wgpu::BufferUsages) -> Self {
        self.usage = usage;
        self
    }
}

impl<'a, T: NoUninit + Zeroable, S: BuilderState> UniformBuilder<'a, T, S> {
    /// Adds initial zero'd date to the uniform
    /// This will reset any previous data
    /// The data is not sent to the GPU until UniformBuilder::build is called
    pub fn with_zero_data(self) -> UniformBuilder<'a, T, <S as sealed::Sealed>::AddData> {
        UniformBuilder {
            bind_group_layout: self.bind_group_layout,
            usage: self.usage,
            label: self.label,
            data: Some(T::zeroed()),
            state: PhantomData,
        }
    }
}
