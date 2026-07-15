//! The lab: registry index + fullscreen phunctor viewer.
//!
//! Viewer contract (inherited from the old phunction.sh labs, kept
//! deliberately): one URL = one visual, chromeless, projector-ready.
//! `f` toggles fullscreen. Pointer position feeds the modulation bus.

use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use phunction_gfx::REGISTRY;

/// `/lab` — the index grid.
#[component]
pub fn LabIndex() -> impl IntoView {
    view! {
        <main class="hero">
            <h1>"the lab"</h1>
            <p class="tagline">"⟨ one URL, one visual — plug into a projector and leave it running ⟩"</p>
            <div class="portals">
                {REGISTRY
                    .iter()
                    .map(|def| {
                        let m = def.meta;
                        view! {
                            <A href=format!("/lab/{}", m.id) attr:class="portal">
                                <span class="portal-glyph">{m.glyph}</span>
                                <span class="portal-name">{m.name}</span>
                                <span class="portal-desc">{m.tagline}</span>
                            </A>
                        }
                    })
                    .collect_view()}
            </div>
        </main>
    }
}

/// `/lab/:id` — fullscreen viewer.
#[component]
pub fn LabView() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.read().get("id").unwrap_or_default();
    let error = RwSignal::new(None::<String>);

    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    #[cfg(target_arch = "wasm32")]
    {
        let started = std::cell::Cell::new(false);
        Effect::new(move |_| {
            let Some(canvas) = canvas_ref.get() else {
                return;
            };
            if started.replace(true) {
                return;
            }
            let Some(def) = phunction_gfx::find(&id()) else {
                error.set(Some(format!("no phunctor named “{}”", id())));
                return;
            };
            wiring::run(canvas, def, error);
        });
    }

    view! {
        <div class="labview">
            {move || {
                error
                    .get()
                    .map(|e| view! { <p class="gfx-error">"✗ " {e}</p> })
            }}
            <canvas node_ref=canvas_ref class="labcanvas"></canvas>
            <div class="labhud">
                <A href="/lab">"← lab"</A>
                <span>{id}</span>
                <span class="hint">"move pointer to modulate · f fullscreen"</span>
            </div>
        </div>
    }
}

#[cfg(target_arch = "wasm32")]
mod wiring {
    use super::*;
    use phunction_gfx::{FrameInput, GfxContext, PhunctorDef};
    use std::cell::Cell;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    /// Bring up wgpu on `canvas` and run `def` until the canvas leaves the DOM.
    pub fn run(
        canvas: web_sys::HtmlCanvasElement,
        def: &'static PhunctorDef,
        error: RwSignal<Option<String>>,
    ) {
        // Pointer → modulation bus (mod0 = x, mod1 = y; 2/3 default mid).
        let mods = Rc::new(Cell::new([0.5f32; 4]));
        {
            let mods = Rc::clone(&mods);
            let target = canvas.clone();
            let on_move = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(
                move |ev: web_sys::PointerEvent| {
                    let w = f64::from(target.client_width()).max(1.0);
                    let h = f64::from(target.client_height()).max(1.0);
                    let mut m = mods.get();
                    m[0] = (f64::from(ev.offset_x()) / w).clamp(0.0, 1.0) as f32;
                    m[1] = 1.0 - (f64::from(ev.offset_y()) / h).clamp(0.0, 1.0) as f32;
                    mods.set(m);
                },
            );
            canvas
                .add_event_listener_with_callback("pointermove", on_move.as_ref().unchecked_ref())
                .expect("pointermove listener");
            on_move.forget();
        }
        // `f` → fullscreen.
        {
            let target = canvas.clone();
            let on_key = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
                move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "f" {
                        let _ = target.request_fullscreen();
                    }
                },
            );
            web_sys::window()
                .expect("window")
                .add_event_listener_with_callback("keydown", on_key.as_ref().unchecked_ref())
                .expect("keydown listener");
            on_key.forget();
        }

        leptos::task::spawn_local(async move {
            let mut ctx = match GfxContext::from_canvas(canvas.clone()).await {
                Ok(c) => c,
                Err(e) => {
                    error.set(Some(e.to_string()));
                    return;
                }
            };
            tracing::info!(backend = ctx.backend(), "gfx up");
            let mut phunctor = (def.create)(&ctx);
            let t0 = web_time::Instant::now();

            crate::raf::raf_loop(move || {
                // Stop when the canvas is detached (route change).
                if !canvas.is_connected() {
                    return false;
                }
                let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let size = (
                    (f64::from(canvas.client_width()) * dpr) as u32,
                    (f64::from(canvas.client_height()) * dpr) as u32,
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
                    // Timeout/occluded/lost: skip this frame, retry next.
                    _ => {
                        ctx.configure(ctx.size);
                        return true;
                    }
                };
                let view = frame.texture.create_view(&Default::default());
                let input = FrameInput {
                    time: t0.elapsed().as_secs_f32(),
                    aspect: size.0 as f32 / size.1 as f32,
                    mods: mods.get(),
                };
                phunctor.frame(&ctx, &view, &input);
                ctx.queue.present(frame);
                true
            });
        });
    }
}
