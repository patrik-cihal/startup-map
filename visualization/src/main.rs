use std::time::Duration;

use dioxus::logger::tracing::{error, info};
use dioxus::prelude::*;
use dioxus_elements::geometry::WheelDelta;
use fastembed::TextEmbedding;
use serde::{Deserialize, Serialize};
use serde_json;

const STARTUPS_JSON: &str = include_str!("../../embedding/startups.json");

#[derive(Serialize, Deserialize, Debug, Clone)]
struct StartupWithPos {
    link: String,
    name: String,
    tagline: String,
    pos_x: f32,
    pos_y: f32,
    team_size: u32,
    logo_url: String,
    embedding: Vec<f32>,
}

fn main() {
    launch(app);
}

#[component]
fn app() -> Element {
    let startups =
        use_signal(|| serde_json::from_str::<Vec<StartupWithPos>>(STARTUPS_JSON).unwrap());

    let mut search_text = use_signal(|| String::new());
    let mut similarities = use_signal(|| vec![1.0; startups.len()]);

    let startups_len = startups.len();
    use_effect(move || {
        let search = search_text();
        info!(search);
        if search.is_empty() {
            similarities.set(vec![1.0; startups_len]);
        } else {
            let mut model = TextEmbedding::try_new(Default::default()).unwrap();
            let search_vec = model.embed(vec![search], None).unwrap()[0].clone();
            let new_similarities = startups
                .iter()
                .map(|s| {
                    let dot: f32 = s
                        .embedding
                        .iter()
                        .zip(&search_vec)
                        .map(|(a, b)| a * b)
                        .sum();
                    let norm_s = s.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
                    let norm_search = search_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
                    let similarity = dot / (norm_s * norm_search);
                    similarity
                })
                .collect::<Vec<f32>>();

            let min_similarity = new_similarities
                .iter()
                .fold(f32::INFINITY, |a, &b| a.min(b));
            let max_similarity = new_similarities
                .iter()
                .fold(f32::NEG_INFINITY, |a, &b| a.max(b));
            let normalized_similarities = new_similarities
                .into_iter()
                .map(|sim| {
                    if max_similarity > min_similarity {
                        (sim - min_similarity) / (max_similarity - min_similarity)
                    } else {
                        0.0
                    }
                })
                .collect::<Vec<f32>>();

            similarities.set(normalized_similarities);
        }
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

    let mut left = use_signal(|| 0);
    let mut right = use_signal(|| startups_len);
    let mut abs_min_team_size = use_signal(|| 50);

    let min_team_size = use_memo(move || {
        let current_zoom = zoom();
        match current_zoom {
            z if z < 0.8 => 20000,    // Large companies
            z if z < 1.5 => 4000,     // Large companies
            z if z < 2.4 => 2500,     // Medium-large companies
            z if z < 5.0 => 1500,     // Medium companies
            z if z < 10.0 => 500,     // Small-medium companies
            z if z < 20.0 => 250,     // Small-medium companies
            z if z < 30.0 => 100,     // Small-medium companies
            z if z < 40.0 => 50,      // Small-medium companies
            z if z < 50.0 => 25,      // Small-medium companies
            _ => abs_min_team_size(), // All companies when very zoomed in
        }
    });

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

            tokio::time::sleep(Duration::from_millis(32)).await; // ~60 FPS
        }
    });

    rsx! {
        document::Title { "Startup Map" }
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

            div {
                style: "transform-origin: 0 0; transform: translate({offset_x()}px, {offset_y()}px); width: 100%; height: 100%;",
                for (i, (startup, similarity)) in startups().into_iter().zip(similarities()).rev().enumerate().take(right()).skip(left())
                {
                    if startup.team_size >= min_team_size() {
                        div {
                            style: "position: absolute; left: {startup.pos_x * 100.0 * zoom()}%; top: {startup.pos_y * 100.0 * zoom()}%; transform: translate(-50%, -50%);",
                            // Logo image as clickable link
                            img {
                                src: "{startup.logo_url}",
                                style: "width: {(30.0 + ((startup.team_size+1) as f32).log10() * 5.0).min(50.0)}px; height: auto; display: block; margin: 0 auto 2px auto; border-radius: 4px;",
                                alt: "{startup.name} logo"
                            }
                            p {
                                style: "margin: 0; font-size: {(12.0 + ((startup.team_size+1) as f32).log10() * 2.0).min(24.0)}px; color: #333; white-space: nowrap;",
                                a {
                                    href: "{startup.link}",
                                    target: "_blank",
                                    strong { "{startup.name} [{i}]" }
                                }
                                ": {startup.tagline}"
                            }
                        }
                    } else if startup.team_size >= abs_min_team_size() {
                        div {
                            style: "position: absolute; left: {startup.pos_x * 100.0 * zoom()}%; top: {startup.pos_y * 100.0 * zoom()}%; transform: translate(-50%, -50%); width: {((startup.team_size as f32).log10() * 2.0).max(2.0).min(8.0)}px; height: {((startup.team_size as f32).log10() * 2.0).max(2.0).min(8.0)}px; background-color: rgba(0, 0, 0, {similarity.powf(3.0)}); border-radius: 50%;",

                        }
                    }
                }
            }
            div {
                style: "position: absolute; display: flex; flex-direction: column; gap: 5px; top: 20px; left: 20px;",
                div {
                    p { "Search Startups Semantically" }
                    input {
                        r#type: "text",
                        placeholder: "Food delivery startup",
                        value: search_text(),
                        onchange: move |ev| search_text.set(ev.value()),
                    }
                }
                div {
                    p { "Start Index" }
                    input {
                        value: left(),
                        oninput: move |ev| {
                            if let Ok(value) = ev.value().parse::<usize>() {
                                left.set(value);
                            }
                        }
                    }
                }
                div {
                    p { "End Index" }
                    input {
                        value: right(),
                        oninput: move |ev| {
                            if let Ok(value) = ev.value().parse::<usize>() {
                                right.set(value);
                            }
                        }
                    }
                }
                div {
                    p { "Minimum Team Size" }
                    input {
                        value: abs_min_team_size(),
                        oninput: move |ev| {
                            if let Ok(value) = ev.value().parse::<u32>() {
                                abs_min_team_size.set(value);
                            }
                        }
                    }
                }
            }

        }
    }
}
