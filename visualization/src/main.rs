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
    let mut is_dragging = use_signal(|| false);
    let mut last_mouse_x = use_signal(|| 0.0f32);
    let mut last_mouse_y = use_signal(|| 0.0f32);

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
                    offset_x.set(offset_x() + dx);
                    offset_y.set(offset_y() + dy);
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
                let old_zoom = *zoom.read();
                let old_offset_x = *offset_x.read();
                let old_offset_y = *offset_y.read();

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

                let new_zoom = (old_zoom * zoom_factor).clamp(0.1, 40.0);

                // Calculate new offsets to zoom towards cursor position
                // Convert mouse position to world coordinates
                let world_x = (mouse_x - old_offset_x) / old_zoom;
                let world_y = (mouse_y - old_offset_y) / old_zoom;

                // Calculate new offset to keep world point under cursor
                let new_offset_x = mouse_x - world_x * new_zoom;
                let new_offset_y = mouse_y - world_y * new_zoom;

                zoom.set(new_zoom);
                offset_x.set(new_offset_x);
                offset_y.set(new_offset_y);
            },

            div {
                style: "transform-origin: 0 0; transform: translate({offset_x()}px, {offset_y()}px); width: 100%; height: 100%;",
                for startup in startups.into_iter().filter(|startup| {
                    let current_zoom = *zoom.read();
                    match current_zoom {
                        z if z < 0.5 => startup.team_size >= 100,  // Only very large companies
                        z if z < 1.0 => startup.team_size >= 50,   // Large companies
                        z if z < 2.0 => startup.team_size >= 20,   // Medium-large companies
                        z if z < 5.0 => startup.team_size >= 10,   // Medium companies
                        z if z < 10.0 => startup.team_size >= 5,   // Small-medium companies
                        _ => true,                                  // All companies when very zoomed in
                    }
                }) {
                    div {
                        style: "position: absolute; left: {startup.pos_x * 100.0 * zoom()}%; top: {startup.pos_y * 100.0 * zoom()}%; transform: translate(-50%, -50%);",
                        // Circle point to show exact position
                        div {
                            style: "width: 3px; height: 3px; border-radius: 50%; background: #000; margin: 0 auto 2px auto;",
                        }
                        p {
                            style: "margin: 0; font-size: 10px; color: #333; white-space: nowrap;",
                            "{startup.tagline}"
                        }
                    }
                }
            }
        }
    }
}
