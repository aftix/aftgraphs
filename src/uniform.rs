use crate::render::Renderer;
use crate::ui::UiPlatform;
use bytemuck::NoUninit;
use std::ops::{Deref, DerefMut};
use wgpu::RenderPass;

mod builder;
pub use builder::UniformBuilder;

pub struct Uniform<T: NoUninit> {
    buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    data: T,
}

pub struct UniformGuard<'a, 'b, T: NoUninit, P: UiPlatform> {
    uniform: &'a mut Uniform<T>,
    renderer: &'a Renderer<'b, P>,
    changed: bool,
}

impl<T: NoUninit> Uniform<T> {
    /// Create a guard to modify the uniform
    /// When the guard drops, it will buffer the data to the GPU
    pub fn modify<'a, 'b, P: UiPlatform>(
        &'a mut self,
        renderer: &'a Renderer<'b, P>,
    ) -> UniformGuard<'a, 'b, T, P> {
        UniformGuard {
            uniform: self,
            renderer,
            changed: false,
        }
    }

    /// Get the bind group (used for set_bind_group on a render pass)
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Get the bind group layout (useful for setting up render pipelines)
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
}

impl<T: NoUninit + PartialEq> Uniform<T> {
    /// Update the uniform value
    /// Will immediately buffer data to the GPU, but only if the
    /// new value is not equal to the old value
    pub fn update<P: UiPlatform>(&mut self, renderer: &Renderer<P>, value: T) {
        if value == self.data {
            self.data = value;
            return;
        }

        self.data = value;
        renderer
            .queue
            .write_buffer(&self.buffer, 0, bytemuck::bytes_of(&self.data));
    }

    pub fn bind<'a, 'b: 'a>(&'b mut self, render_pass: &mut RenderPass<'a>, slot: u32) {
        render_pass.set_bind_group(slot, self.bind_group(), &[]);
    }
}

impl<T: NoUninit> AsRef<T> for Uniform<T> {
    fn as_ref(&self) -> &T {
        &self.data
    }
}

impl<T: NoUninit> Deref for Uniform<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: NoUninit, P: UiPlatform> AsRef<T> for UniformGuard<'_, '_, T, P> {
    fn as_ref(&self) -> &T {
        self.uniform.as_ref()
    }
}

/// Using this will make the data be sent to the GPU on drop
impl<T: NoUninit, P: UiPlatform> AsMut<T> for UniformGuard<'_, '_, T, P> {
    fn as_mut(&mut self) -> &mut T {
        self.changed = true;
        &mut self.uniform.data
    }
}

impl<T: NoUninit, P: UiPlatform> Deref for UniformGuard<'_, '_, T, P> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.uniform.deref()
    }
}

/// Using this will make the data be sent to the GPU on drop
impl<T: NoUninit, P: UiPlatform> DerefMut for UniformGuard<'_, '_, T, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

/// Buffers data to GPU if changed
impl<T: NoUninit, P: UiPlatform> Drop for UniformGuard<'_, '_, T, P> {
    fn drop(&mut self) {
        if self.changed {
            self.renderer.queue.write_buffer(
                &self.uniform.buffer,
                0,
                bytemuck::bytes_of(&self.uniform.data),
            );
        }
    }
}
