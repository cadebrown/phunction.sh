//! The topbar: one dense strip owns the chrome — transport, tempo,
//! minds, worlds, zen. The qualia move: everything a live set reaches
//! for every minute lives HERE, in one row, so the panes below can stay
//! folded and the field stays visible.

use crate::phazor_panel::{wiring, Cv, Meters};
use crate::presets::{apply_preset, PRESETS};
use leptos::prelude::*;
use phazor_core::Command;

/// Toggle zen on the document root (shared by the topbar button, the
/// floating exit handle, and the `z` key).
pub(crate) fn toggle_zen() {
    #[cfg(target_arch = "wasm32")]
    if let Some(root) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    {
        let _ = root.class_list().toggle("zen");
    }
}

/// The chrome strip. Owns no state — every signal is lifted from the
/// page, so worlds/minds/tempo stay one source of truth.
#[component]
pub fn Topbar(
    /// Engine telemetry (beat/vox/drop LCDs, play lit-state).
    meters: RwSignal<Meters>,
    /// User tempo (the LCD shows the SET tempo; the era weather breathes
    /// the effective rate around it).
    tempo: RwSignal<f32>,
    /// Active mind id (lit state + switcher).
    mind: RwSignal<&'static str>,
    /// The sequencer pattern, rewritten by worlds.
    steps: RwSignal<[Option<(u8, u8)>; 16]>,
    /// Viewport params, rewritten by worlds.
    citadel: RwSignal<crate::fractal::CitadelParams>,
    /// Every continuous control's UI-side truth.
    cv: Cv,
) -> impl IntoView {
    view! {
        <header class="phz-topbar">
            <div class="phz-titlerow">
                <div class="phz-left">
                    <span class="phz-name">"phazor"</span>
                    <span class="phz-lcd">{move || format!("beat {:>7.2}", meters.get().beats)}</span>
                    <span class="phz-lcd">{move || format!("vox {:>2}", meters.get().voices)}</span>
                    <span class="phz-lcd" class:warn={move || meters.get().dropped > 0}>
                        {move || format!("drop {}", meters.get().dropped)}
                    </span>
                </div>
                <div class="phz-corner">
                    <button class="ctrl-btn" on:click=move |_| toggle_zen()>"zen"</button>
                </div>
            </div>
            <div class="phz-controls">
                <button
                    class="ctrl-btn hot"
                    class:lit=move || meters.get().playing
                    on:click=move |_| wiring::send(Command::Play)
                >"▶"</button>
                <button class="ctrl-btn" on:click=move |_| wiring::send(Command::Stop)>"■"</button>
                <button class="ctrl-btn panic" on:click=move |_| wiring::send(Command::AllNotesOff)>"✕"</button>
                <span class="phz-sep"></span>
                <button class="ctrl-btn" on:click=move |_| {
                    let t = (tempo.get_untracked() - 4.0).max(50.0);
                    tempo.set(t);
                    wiring::send(Command::SetTempo(f64::from(t)));
                }>"−"</button>
                <span class="phz-lcd">{move || format!("{:>3.0} bpm", tempo.get())}</span>
                <button class="ctrl-btn" on:click=move |_| {
                    let t = (tempo.get_untracked() + 4.0).min(200.0);
                    tempo.set(t);
                    wiring::send(Command::SetTempo(f64::from(t)));
                }>"+"</button>
                <span class="phz-sep"></span>
                {crate::fractal::MINDS.map(|(id, label, _)| view! {
                    <button
                        class="ctrl-btn"
                        class:lit=move || mind.get() == id
                        on:click=move |_| crate::fractal::request_mind(id)
                    >{label}</button>
                })}
                <span class="phz-sep"></span>
                {PRESETS.iter().map(|preset| {
                    let name = preset.name;
                    view! {
                        <button
                            class="ctrl-btn world"
                            on:click=move |_| apply_preset(preset, steps, citadel, tempo, cv)
                        >{name}</button>
                    }
                }).collect_view()}
            </div>
        </header>
    }
}
