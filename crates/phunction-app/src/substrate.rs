//! The substrate: every page floats on a slow domain-colored field.
//! Progressive enhancement — if the GPU says no, the flat plane remains and
//! nobody is told. Rendered at half resolution on purpose: cheaper, and the
//! softness is the look.

use leptos::prelude::*;

/// Site-wide living background. Mounted once in `App`, fixed at z-0.
#[component]
pub fn Substrate() -> impl IntoView {
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    #[cfg(target_arch = "wasm32")]
    {
        use std::cell::Cell;
        let started = Cell::new(false);
        Effect::new(move |_| {
            let Some(canvas) = canvas_ref.get() else {
                return;
            };
            if started.replace(true) {
                return;
            }
            wiring::run(canvas);
        });
    }

    view! { <canvas node_ref=canvas_ref class="substrate" aria-hidden="true"></canvas> }
}

#[cfg(target_arch = "wasm32")]
mod wiring {
    use phunction_gfx::{FrameInput, GfxContext, Phunctor as _};

    pub fn run(canvas: web_sys::HtmlCanvasElement) {
        let reduced = web_sys::window()
            .and_then(|w| {
                w.match_media("(prefers-reduced-motion: reduce)")
                    .ok()
                    .flatten()
            })
            .is_some_and(|m| m.matches());

        leptos::task::spawn_local(async move {
            let mut ctx = match GfxContext::from_canvas(canvas.clone()).await {
                Ok(c) => c,
                Err(e) => {
                    // the flat plane is a fine sky; log and move on
                    tracing::info!("substrate unavailable: {e}");
                    return;
                }
            };
            let mut field = phunction_gfx::substrate(&ctx);
            let t0 = web_time::Instant::now();
            let mut rendered_once = false;

            crate::raf::raf_loop(move || {
                if !canvas.is_connected() {
                    return false;
                }
                if reduced && rendered_once {
                    return false; // one still frame, then rest
                }
                let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let size = (
                    (f64::from(canvas.client_width()) * dpr * 0.5).max(1.0) as u32,
                    (f64::from(canvas.client_height()) * dpr * 0.5).max(1.0) as u32,
                );
                if size.0 == 0 || size.1 == 0 {
                    return true;
                }
                if (canvas.width(), canvas.height()) != size {
                    canvas.set_width(size.0);
                    canvas.set_height(size.1);
                }
                ctx.resize_if_needed(size);

                use phunction_gfx::wgpu::CurrentSurfaceTexture as Cst;
                let frame = match ctx.surface.get_current_texture() {
                    Cst::Success(f) => f,
                    Cst::Suboptimal(f) => {
                        ctx.configure(ctx.size);
                        f
                    }
                    _ => {
                        ctx.configure(ctx.size);
                        return true;
                    }
                };
                let view = frame
                    .texture
                    .create_view(&phunction_gfx::wgpu::TextureViewDescriptor::default());
                let input = FrameInput {
                    time: t0.elapsed().as_secs_f32(),
                    aspect: size.0 as f32 / size.1 as f32,
                    // mod2 = intensity: visible weather, legible ink
                    mods: [0.5, 0.5, 0.55, 0.5],
                };
                field.frame(&ctx, &view, &input);
                ctx.queue.present(frame);
                rendered_once = true;
                true
            });
        });
    }
}
