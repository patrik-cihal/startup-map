use dioxus::logger::tracing::{error, info};
use dioxus::prelude::*;
use dioxus_elements::geometry::WheelDelta;
use serde::{Deserialize, Serialize};

const STARTUPS_CSV: &str = include_str!("../../embedding/startups.csv");

#[derive(Serialize, Deserialize, Debug, Clone)]
struct StartupWithPos {
    name: String,
    tagline: String,
    pos_x: f32,
    pos_y: f32,
    team_size: u32,
    logo_url: String,
}

fn main() {
    launch(app);
}

#[component]
fn app() -> Element {
    let startups = use_hook(|| {
        csv::Reader::from_reader(STARTUPS_CSV.as_bytes())
            .deserialize::<StartupWithPos>()
            .map(|res| res.unwrap())
            .collect::<Vec<_>>()
    });

    let mut zoom = use_signal(|| 1.0f32);
    let mut offset_x = use_signal(|| 0.0f32);
    let mut offset_y = use_signal(|| 0.0f32);
    let mut target_zoom = use_signal(|| 1.0f32);
    let mut target_offset_x = use_signal(|| 0.0f32);
    let mut target_offset_y = use_signal(|| 0.0f32);
    let mut is_dragging = use_signal(|| false);
    let mut last_mouse_x = use_signal(|| 0.0f32);
    let mut last_mouse_y = use_signal(|| 0.0f32);

    let min_team_size = use_memo(move || {
        let current_zoom = zoom();
        match current_zoom {
            z if z < 1.0 => 4000, // Large companies
            z if z < 2.0 => 3000, // Medium-large companies
            z if z < 5.0 => 1500, // Medium companies
            z if z < 10.0 => 500, // Small-medium companies
            z if z < 20.0 => 250, // Small-medium companies
            z if z < 30.0 => 100, // Small-medium companies
            z if z < 40.0 => 50,  // Small-medium companies
            z if z < 50.0 => 25,  // Small-medium companies
            _ => 0,               // All companies when very zoomed in
        }
    });

    info!("{zoom} {min_team_size}");

    // Smooth animation loop
    use_future(move || async move {
        loop {
            let current_zoom = *zoom.read();
            let current_offset_x = *offset_x.read();
            let current_offset_y = *offset_y.read();

            let target_zoom_val = *target_zoom.read();
            let target_offset_x_val = *target_offset_x.read();
            let target_offset_y_val = *target_offset_y.read();

            // Check if we need to animate
            let zoom_diff = (target_zoom_val - current_zoom).abs();
            let x_diff = (target_offset_x_val - current_offset_x).abs();
            let y_diff = (target_offset_y_val - current_offset_y).abs();

            if zoom_diff > 0.001 || x_diff > 0.1 || y_diff > 0.1 {
                let lerp_factor = 0.48; // Adjust for animation speed

                let new_zoom = current_zoom + (target_zoom_val - current_zoom) * lerp_factor;
                let new_offset_x =
                    current_offset_x + (target_offset_x_val - current_offset_x) * lerp_factor;
                let new_offset_y =
                    current_offset_y + (target_offset_y_val - current_offset_y) * lerp_factor;

                zoom.set(new_zoom);
                offset_x.set(new_offset_x);
                offset_y.set(new_offset_y);
            }

            gloo_timers::future::TimeoutFuture::new(32).await; // ~60 FPS
        }
    });

    rsx! {
        div {
            style: "width: 100vw; height: 100vh; position: relative; overflow: hidden; margin: 0; padding: 0; cursor: grab;",
            onmousedown: move |evt| {
                is_dragging.set(true);
                last_mouse_x.set(evt.client_coordinates().x as f32);
                last_mouse_y.set(evt.client_coordinates().y as f32);
            },
            onmousemove: move |evt| {
                if *is_dragging.read() {
                    let current_x = evt.client_coordinates().x as f32;
                    let current_y = evt.client_coordinates().y as f32;
                    let dx = current_x - *last_mouse_x.read();
                    let dy = current_y - *last_mouse_y.read();
                    let new_x = offset_x() + dx;
                    let new_y = offset_y() + dy;
                    offset_x.set(new_x);
                    offset_y.set(new_y);
                    target_offset_x.set(new_x);
                    target_offset_y.set(new_y);
                    last_mouse_x.set(current_x);
                    last_mouse_y.set(current_y);
                }
            },
            onmouseup: move |_| {
                is_dragging.set(false);
            },
            onwheel: move |evt| {
                evt.prevent_default();
                let mouse_x = evt.client_coordinates().x as f32;
                let mouse_y = evt.client_coordinates().y as f32;
                let old_zoom = *target_zoom.read();
                let old_offset_x = *target_offset_x.read();
                let old_offset_y = *target_offset_y.read();

                let delta = evt.data.delta();
                let zoom_factor = match delta {
                    WheelDelta::Pixels(vector) => {
                        let delta_y = vector.y as f32;
                        if delta_y < 0.0 { 1.1 } else { 0.9 }
                    },
                    WheelDelta::Lines(vector) => {
                        let delta_y = vector.y as f32;
                        if delta_y < 0.0 { 1.1 } else { 0.9 }
                    },
                    WheelDelta::Pages(vector) => {
                        let delta_y = vector.y as f32;
                        if delta_y < 0.0 { 1.1 } else { 0.9 }
                    }
                };

                let new_zoom = (old_zoom * zoom_factor).clamp(0.1, 60.0);

                // Calculate new offsets to zoom towards cursor position
                // Convert mouse position to world coordinates
                let world_x = (mouse_x - old_offset_x) / old_zoom;
                let world_y = (mouse_y - old_offset_y) / old_zoom;

                // Calculate new offset to keep world point under cursor
                let new_offset_x = mouse_x - world_x * new_zoom;
                let new_offset_y = mouse_y - world_y * new_zoom;

                target_zoom.set(new_zoom);
                target_offset_x.set(new_offset_x);
                target_offset_y.set(new_offset_y);
            },

            {info!("{}", min_team_size())}
            div {
                style: "transform-origin: 0 0; transform: translate({offset_x()}px, {offset_y()}px); width: 100%; height: 100%;",
                for startup in startups.into_iter()
                {
                    if startup.team_size >= min_team_size() {
                        div {
                            style: "position: absolute; left: {startup.pos_x * 100.0 * zoom()}%; top: {startup.pos_y * 100.0 * zoom()}%; transform: translate(-50%, -50%);",
                            // Logo image
                            img {
                                src: "{startup.logo_url}",
                                style: "width: {(30.0 + ((startup.team_size+1) as f32).log10() * 5.0).min(50.0)}px; height: auto; display: block; margin: 0 auto 2px auto; border-radius: 4px;",
                                alt: "{startup.name} logo"
                            }
                            p {
                                style: "margin: 0; font-size: {(12.0 + ((startup.team_size+1) as f32).log10() * 2.0).min(24.0)}px; color: #333; white-space: nowrap;",
                                strong { "{startup.name}" }
                                ": {startup.tagline}"
                            }
                        }
                    }
                }
            }
        }
    }
}
