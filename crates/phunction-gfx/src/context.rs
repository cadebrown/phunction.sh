//! wgpu bring-up: canvas → surface → adapter → device.

use thiserror::Error;

/// Everything a phunctor needs to draw.
pub struct GfxContext {
    /// The wgpu device.
    pub device: wgpu::Device,
    /// The submission queue.
    pub queue: wgpu::Queue,
    /// The canvas-backed surface.
    pub surface: wgpu::Surface<'static>,
    /// Chosen surface format (first capability — native preference order).
    pub format: wgpu::TextureFormat,
    /// Current surface size in physical pixels.
    pub size: (u32, u32),
    adapter: wgpu::Adapter,
}

/// Why GPU bring-up failed. Every variant is shown verbatim in the UI —
/// "radical art software" owes the user a real reason, not a sad spinner.
#[derive(Debug, Error)]
pub enum GfxError {
    /// Surface creation failed (canvas already claimed, or context lost).
    #[error("could not create surface from canvas: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    /// No adapter — neither WebGPU nor WebGL2 available.
    #[error("no GPU adapter (WebGPU off and WebGL2 unavailable): {0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),
    /// Device request failed (limits/features mismatch).
    #[error("adapter refused device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
}

impl GfxContext {
    /// Bring up wgpu on a canvas. Tries WebGPU first, falls back to WebGL2.
    ///
    /// # Errors
    /// Returns [`GfxError`] when no usable adapter/device exists; the caller
    /// surfaces the message to the user.
    #[cfg(target_arch = "wasm32")]
    pub async fn from_canvas(canvas: web_sys::HtmlCanvasElement) -> Result<Self, GfxError> {
        // Probes for real WebGPU support and only then keeps BROWSER_WEBGPU —
        // a plain Instance::new with BROWSER_WEBGPU set would refuse to hand
        // out GL adapters even when WebGPU is broken (wgpu docs' own caveat).
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL;
        let instance = wgpu::util::new_instance_with_webgpu_detection(desc).await;
        let size = (canvas.width().max(1), canvas.height().max(1));
        let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await?;
        let format = surface.get_capabilities(&adapter).formats[0];
        let ctx = Self {
            device,
            queue,
            surface,
            format,
            size,
            adapter,
        };
        ctx.configure(size);
        Ok(ctx)
    }

    /// Which backend actually won (shown in the debug HUD: "webgpu"/"gl").
    #[must_use]
    pub fn backend(&self) -> &'static str {
        match self.adapter.get_info().backend {
            wgpu::Backend::BrowserWebGpu => "webgpu",
            wgpu::Backend::Gl => "webgl2",
            _ => "native",
        }
    }

    /// (Re)configure the swapchain for `size` physical pixels.
    pub fn configure(&self, size: (u32, u32)) {
        self.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.format,
                width: size.0.max(1),
                height: size.1.max(1),
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
                color_space: wgpu::SurfaceColorSpace::default(),
            },
        );
    }

    /// Resize if needed (call each frame with the canvas's physical size —
    /// reconfiguring only on change keeps this free).
    pub fn resize_if_needed(&mut self, size: (u32, u32)) {
        if size != self.size && size.0 > 0 && size.1 > 0 {
            self.size = size;
            self.configure(size);
        }
    }
}
