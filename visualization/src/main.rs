use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;
use dioxus_elements::geometry::WheelDelta;
#[cfg(not(target_arch = "wasm32"))]
use fastembed::TextEmbedding;
#[cfg(not(target_arch = "wasm32"))]
use keyboard_types::Key;
use serde::{Deserialize, Serialize};

const STARTUPS_JSON: &str = include_str!("../../embedding/startups.json");

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    List,
    Map,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, PartialEq)]
enum SortMode {
    Similarity,
    TeamSize,
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        dioxus::LaunchBuilder::new()
            .with_cfg(
                Config::new()
                    .with_menu(None)
                    .with_window(WindowBuilder::new().with_title("Startup Map")),
            )
            .launch(app);
    }
    #[cfg(target_arch = "wasm32")]
    {
        dioxus::launch(app);
    }
}

async fn sleep_ms(ms: u64) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }
    #[cfg(target_arch = "wasm32")]
    {
        gloo_timers::future::sleep(Duration::from_millis(ms)).await;
    }
}

#[component]
fn app() -> Element {
    let startups =
        use_signal(|| serde_json::from_str::<Vec<StartupWithPos>>(STARTUPS_JSON).unwrap());

    // Search signals (desktop only)
    #[cfg(not(target_arch = "wasm32"))]
    let search_query = use_signal(|| String::new());
    #[cfg(not(target_arch = "wasm32"))]
    let committed_search = use_signal(|| String::new());
    #[cfg(not(target_arch = "wasm32"))]
    let mut similarities = use_signal(|| vec![1.0f32; startups.len()]);
    #[cfg(not(target_arch = "wasm32"))]
    let mut is_searching = use_signal(|| false);
    #[cfg(not(target_arch = "wasm32"))]
    let view_mode = use_signal(|| ViewMode::List);
    #[cfg(not(target_arch = "wasm32"))]
    let sort_mode = use_signal(|| SortMode::TeamSize);
    #[cfg(not(target_arch = "wasm32"))]
    let min_team_size_filter = use_signal(|| 10u32);
    #[cfg(not(target_arch = "wasm32"))]
    let min_similarity_filter = use_signal(|| 0.7f32);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let startups_len = startups.len();
        use_effect(move || {
            let search = committed_search();
            if search.is_empty() {
                similarities.set(vec![1.0; startups_len]);
                is_searching.set(false);
            } else {
                is_searching.set(true);
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
                        dot / (norm_s * norm_search)
                    })
                    .collect::<Vec<f32>>();

                let min_sim = new_similarities.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                let max_sim = new_similarities.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                let range = max_sim - min_sim;
                let normalized = new_similarities
                    .into_iter()
                    .map(|sim| if range > 0.0 { (sim - min_sim) / range } else { 0.0 })
                    .collect::<Vec<f32>>();

                similarities.set(normalized);
                is_searching.set(false);
            }
        });
    }

    // Zoom/pan state
    let mut zoom = use_signal(|| 1.0f32);
    let mut offset_x = use_signal(|| 0.0f32);
    let mut offset_y = use_signal(|| 0.0f32);
    let target_zoom = use_signal(|| 1.0f32);
    let target_offset_x = use_signal(|| 0.0f32);
    let target_offset_y = use_signal(|| 0.0f32);
    let is_dragging = use_signal(|| false);
    let last_mouse_x = use_signal(|| 0.0f32);
    let last_mouse_y = use_signal(|| 0.0f32);

    // Smooth animation loop
    use_future(move || async move {
        loop {
            let cz = *zoom.read();
            let cox = *offset_x.read();
            let coy = *offset_y.read();
            let tz = *target_zoom.read();
            let tox = *target_offset_x.read();
            let toy = *target_offset_y.read();

            if (tz - cz).abs() > 0.001 || (tox - cox).abs() > 0.1 || (toy - coy).abs() > 0.1 {
                let f = 0.48;
                zoom.set(cz + (tz - cz) * f);
                offset_x.set(cox + (tox - cox) * f);
                offset_y.set(coy + (toy - coy) * f);
            }

            sleep_ms(32).await;
        }
    });

    #[cfg(not(target_arch = "wasm32"))]
    let main_content = rsx! {
        MainView {
            startups,
            similarities,
            search_query,
            committed_search,
            is_searching,
            sort_mode,
            min_team_size_filter,
            min_similarity_filter,
            view_mode,
            zoom,
            offset_x,
            offset_y,
            target_zoom,
            target_offset_x,
            target_offset_y,
            is_dragging,
            last_mouse_x,
            last_mouse_y,
        }
    };
    #[cfg(target_arch = "wasm32")]
    let main_content = rsx! {
        MapView {
            startups,
            similarities: Signal::new(vec![]),
            sort_by_similarity: false,
            has_search: false,
            min_team_size_filter: 0u32,
            min_similarity_filter: 0.0f32,
            zoom,
            offset_x,
            offset_y,
            target_zoom,
            target_offset_x,
            target_offset_y,
            is_dragging,
            last_mouse_x,
            last_mouse_y,
        }
    };

    rsx! {
        document::Title { "Startup Map" }
        document::Link { rel: "stylesheet", href: asset!("/assets/tailwind.css") }
        document::Link { rel: "stylesheet", href: "https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@300;400;500;600;700&display=swap" }

        div {
            class: "w-screen h-screen m-0 p-0 font-mono flex flex-col overflow-hidden",
            style: "background: #080c08;",

            // Top bar
            div {
                class: "flex items-center justify-between px-6 py-2.5 z-50 shrink-0",
                style: "background: rgba(8, 14, 8, 0.95); border-bottom: 1px solid rgba(0, 255, 170, 0.15);",

                h1 {
                    class: "text-sm font-medium tracking-widest uppercase",
                    style: "color: #00ffaa;",
                    "// startup_map"
                }

                // Status indicator
                div {
                    class: "flex items-center gap-2",
                    div {
                        class: "w-2 h-2 rounded-full",
                        style: "background: #00ffaa; box-shadow: 0 0 6px #00ffaa;",
                    }
                    span {
                        class: "text-xs uppercase tracking-wider",
                        style: "color: #a0a8a4;",
                        "Online"
                    }
                }
            }

            {main_content}
        }
    }
}

#[component]
fn MapView(
    startups: Signal<Vec<StartupWithPos>>,
    similarities: Signal<Vec<f32>>,
    sort_by_similarity: bool,
    has_search: bool,
    min_team_size_filter: u32,
    min_similarity_filter: f32,
    mut zoom: Signal<f32>,
    mut offset_x: Signal<f32>,
    mut offset_y: Signal<f32>,
    mut target_zoom: Signal<f32>,
    mut target_offset_x: Signal<f32>,
    mut target_offset_y: Signal<f32>,
    mut is_dragging: Signal<bool>,
    mut last_mouse_x: Signal<f32>,
    mut last_mouse_y: Signal<f32>,
) -> Element {
    let min_team_size = use_memo(move || {
        match zoom() {
            z if z < 0.8 => 20000,
            z if z < 1.5 => 4000,
            z if z < 2.4 => 2500,
            z if z < 5.0 => 1500,
            z if z < 10.0 => 500,
            z if z < 20.0 => 250,
            z if z < 30.0 => 100,
            z if z < 40.0 => 50,
            _ => 25,
        }
    });

    rsx! {
        div {
            class: "flex-1 relative overflow-hidden cursor-grab",
            style: "background: #060a06;",

            onmousedown: move |evt| {
                is_dragging.set(true);
                last_mouse_x.set(evt.client_coordinates().x as f32);
                last_mouse_y.set(evt.client_coordinates().y as f32);
            },
            onmousemove: move |evt| {
                if *is_dragging.read() {
                    let cx = evt.client_coordinates().x as f32;
                    let cy = evt.client_coordinates().y as f32;
                    let dx = cx - *last_mouse_x.read();
                    let dy = cy - *last_mouse_y.read();
                    let nx = offset_x() + dx;
                    let ny = offset_y() + dy;
                    offset_x.set(nx);
                    offset_y.set(ny);
                    target_offset_x.set(nx);
                    target_offset_y.set(ny);
                    last_mouse_x.set(cx);
                    last_mouse_y.set(cy);
                }
            },
            onmouseup: move |_| {
                is_dragging.set(false);
            },
            onwheel: move |evt| {
                evt.prevent_default();
                let mx = evt.client_coordinates().x as f32;
                let my = evt.client_coordinates().y as f32;
                let oz = *target_zoom.read();
                let oox = *target_offset_x.read();
                let ooy = *target_offset_y.read();

                let zoom_factor = match evt.data.delta() {
                    WheelDelta::Pixels(v) => if (v.y as f32) < 0.0 { 1.1 } else { 0.9 },
                    WheelDelta::Lines(v) => if (v.y as f32) < 0.0 { 1.1 } else { 0.9 },
                    WheelDelta::Pages(v) => if (v.y as f32) < 0.0 { 1.1 } else { 0.9 },
                };

                let nz = (oz * zoom_factor).clamp(0.1, 60.0);
                let wx = (mx - oox) / oz;
                let wy = (my - ooy) / oz;

                target_zoom.set(nz);
                target_offset_x.set(mx - wx * nz);
                target_offset_y.set(my - wy * nz);
            },

            div {
                class: "w-full h-full origin-top-left",
                style: "transform: translate({offset_x()}px, {offset_y()}px);",
                {
                    let sims = similarities();
                    let all_startups = startups();

                    // Build filtered + sorted index list
                    let mut indexed: Vec<(usize, f32)> = all_startups.iter().enumerate()
                        .map(|(i, _)| (i, sims.get(i).copied().unwrap_or(1.0)))
                        .filter(|(i, sim)| {
                            let ts = all_startups[*i].team_size;
                            if sort_by_similarity {
                                ts >= min_team_size_filter
                            } else if has_search {
                                *sim >= min_similarity_filter
                            } else {
                                true
                            }
                        })
                        .collect();

                    // Sort so highest-priority items are last (rendered on top)
                    if sort_by_similarity {
                        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                    } else {
                        indexed.sort_by(|a, b| all_startups[a.0].team_size.cmp(&all_startups[b.0].team_size));
                    }

                    rsx! {
                        for (i, sim) in indexed {
                            {
                                let startup = &all_startups[i];
                                let opacity = 0.06_f32.max(sim.powf(2.0));
                                if startup.team_size >= min_team_size() {
                                    let logo_size = (30.0 + ((startup.team_size + 1) as f32).log10() * 5.0).min(50.0);
                                    let font_size = (12.0 + ((startup.team_size + 1) as f32).log10() * 2.0).min(20.0);
                                    rsx! {
                                        div {
                                            key: "{i}",
                                            class: "absolute -translate-x-1/2 -translate-y-1/2",
                                            style: "left: {startup.pos_x * 100.0 * zoom()}%; top: {startup.pos_y * 100.0 * zoom()}%; opacity: {opacity};",
                                            img {
                                                src: "{startup.logo_url}",
                                                loading: "lazy",
                                                class: "block mx-auto mb-0.5 rounded-md",
                                                style: "width: {logo_size}px; height: auto;",
                                                alt: "{startup.name}"
                                            }
                                            p {
                                                class: "m-0 whitespace-nowrap",
                                                style: "font-size: {font_size}px;",
                                                a {
                                                    href: "{startup.link}",
                                                    target: "_blank",
                                                    class: "font-medium no-underline transition-colors",
                                                    style: "color: #e0e4e2; font-family: 'JetBrains Mono', monospace;",
                                                    "{startup.name}"
                                                }
                                                if opacity > 0.3 {
                                                    span {
                                                        class: "font-normal",
                                                        style: "color: #9a9fa0;",
                                                        " {startup.tagline}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else if startup.team_size >= 25 {
                                    let dot_size = ((startup.team_size as f32).log10() * 2.0).max(2.0).min(8.0);
                                    let dot_alpha = sim.powf(3.0) * 0.6;
                                    rsx! {
                                        div {
                                            key: "{i}",
                                            class: "absolute -translate-x-1/2 -translate-y-1/2 rounded-full",
                                            style: "left: {startup.pos_x * 100.0 * zoom()}%; top: {startup.pos_y * 100.0 * zoom()}%; width: {dot_size}px; height: {dot_size}px; background-color: rgba(0, 255, 170, {dot_alpha});",
                                        }
                                    }
                                } else {
                                    rsx! {}
                                }
                            }
                        }
                    }
                }
            }

        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[component]
fn MainView(
    startups: Signal<Vec<StartupWithPos>>,
    similarities: Signal<Vec<f32>>,
    mut search_query: Signal<String>,
    mut committed_search: Signal<String>,
    is_searching: ReadSignal<bool>,
    mut sort_mode: Signal<SortMode>,
    mut min_team_size_filter: Signal<u32>,
    mut min_similarity_filter: Signal<f32>,
    mut view_mode: Signal<ViewMode>,
    zoom: Signal<f32>,
    offset_x: Signal<f32>,
    offset_y: Signal<f32>,
    target_zoom: Signal<f32>,
    target_offset_x: Signal<f32>,
    target_offset_y: Signal<f32>,
    is_dragging: Signal<bool>,
    last_mouse_x: Signal<f32>,
    last_mouse_y: Signal<f32>,
) -> Element {
    let search = committed_search();
    let has_search = !search.is_empty();
    let is_sim_sort = sort_mode() == SortMode::Similarity;
    let is_list = view_mode() == ViewMode::List;

    // Find max team size for slider range
    let max_team_size = use_memo(move || {
        startups().iter().map(|s| s.team_size).max().unwrap_or(1000)
    });

    rsx! {
        div {
            class: "flex-1 flex flex-col overflow-hidden",

            // Search toolbar
            div {
                class: "flex items-center gap-4 px-6 py-3 shrink-0 flex-wrap",
                style: "border-bottom: 1px solid rgba(255, 255, 255, 0.08); background: rgba(255, 255, 255, 0.02);",

                // Search input
                div {
                    class: "relative w-80",
                    span {
                        class: "absolute left-3 top-1/2 -translate-y-1/2 text-xs",
                        style: "color: #00ffaa;",
                        ">"
                    }
                    input {
                        r#type: "text",
                        placeholder: "query vector search...",
                        value: search_query(),
                        class: "w-full py-2 pl-7 pr-4 text-xs font-mono outline-none transition-colors",
                        style: "background: rgba(255, 255, 255, 0.04); border: 1px solid rgba(255, 255, 255, 0.12); border-radius: 4px; color: #e0e4e2; caret-color: #00ffaa;",
                        oninput: move |ev| search_query.set(ev.value()),
                        onkeydown: move |ev| {
                            if ev.key() == Key::Enter {
                                committed_search.set(search_query());
                            }
                        },
                    }
                    if is_searching() {
                        div {
                            class: "absolute right-3 top-1/2 -translate-y-1/2 text-xs",
                            style: "color: #00ffaa;",
                            "..."
                        }
                    }
                }

                // Sort toggle
                div {
                    class: "flex p-0.5",
                    style: "background: rgba(255, 255, 255, 0.04); border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 4px;",
                    div {
                        class: "px-3 py-1.5 cursor-pointer text-xs font-medium select-none uppercase tracking-wider",
                        style: if !is_sim_sort { "color: #00ffaa; background: rgba(255, 255, 255, 0.06); border-radius: 3px;" } else { "color: #a0a8a4; background: transparent;" },
                        onclick: move |_| sort_mode.set(SortMode::TeamSize),
                        "Team Size"
                    }
                    div {
                        class: if has_search { "px-3 py-1.5 cursor-pointer text-xs font-medium select-none uppercase tracking-wider" } else { "px-3 py-1.5 cursor-not-allowed text-xs font-medium select-none uppercase tracking-wider" },
                        style: if is_sim_sort && has_search { "color: #00ffaa; background: rgba(255, 255, 255, 0.06); border-radius: 3px;" } else if has_search { "color: #a0a8a4; background: transparent;" } else { "color: #505850; background: transparent;" },
                        onclick: move |_| {
                            if has_search {
                                sort_mode.set(SortMode::Similarity);
                            }
                        },
                        "Similarity"
                    }
                }

                // Filter slider
                div {
                    class: "flex items-center gap-2",
                    if is_sim_sort {
                        {
                            let max_ts = max_team_size() as f64;
                            let cur_ts = min_team_size_filter() as f64;
                            // Reverse: slider_pos = 1000 * (team_size / max)^(1/3)
                            let slider_val = if max_ts > 0.0 { (1000.0 * (cur_ts / max_ts).powf(1.0 / 3.0)).round() as u32 } else { 0 };
                            rsx! {
                                span {
                                    class: "text-xs whitespace-nowrap uppercase tracking-wider",
                                    style: "color: #a0a8a4;",
                                    "Min personnel:"
                                }
                                input {
                                    r#type: "range",
                                    min: "0",
                                    max: "1000",
                                    value: "{slider_val}",
                                    class: "w-32",
                                    oninput: move |ev| {
                                        if let Ok(v) = ev.value().parse::<f64>() {
                                            let max_ts = max_team_size() as f64;
                                            let t = v / 1000.0;
                                            let team_size = (max_ts * t * t * t).round() as u32;
                                            min_team_size_filter.set(team_size);
                                        }
                                    },
                                }
                                span {
                                    class: "text-xs whitespace-nowrap uppercase tracking-wider",
                                    style: "color: #a0a8a4;",
                                    "{min_team_size_filter()}"
                                }
                            }
                        }
                    } else {
                        span {
                            class: "text-xs whitespace-nowrap uppercase tracking-wider",
                            style: "color: #a0a8a4;",
                            "Min match:"
                        }
                        input {
                            r#type: "range",
                            min: "0",
                            max: "100",
                            value: "{(min_similarity_filter() * 100.0).round()}",
                            class: "w-32",
                            oninput: move |ev| {
                                if let Ok(v) = ev.value().parse::<f32>() {
                                    min_similarity_filter.set(v / 100.0);
                                }
                            },
                        }
                        span {
                            class: "text-xs whitespace-nowrap uppercase tracking-wider",
                            style: "color: #a0a8a4;",
                            "{(min_similarity_filter() * 100.0).round()}%"
                        }
                    }
                }

                // View toggle
                div {
                    class: "flex p-0.5 ml-auto",
                    style: "background: rgba(255, 255, 255, 0.04); border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 4px;",
                    div {
                        class: "px-3 py-1.5 cursor-pointer text-xs font-medium select-none uppercase tracking-wider",
                        style: if is_list { "color: #00ffaa; background: rgba(255, 255, 255, 0.06); border-radius: 3px;" } else { "color: #a0a8a4; background: transparent;" },
                        onclick: move |_| view_mode.set(ViewMode::List),
                        "List"
                    }
                    div {
                        class: "px-3 py-1.5 cursor-pointer text-xs font-medium select-none uppercase tracking-wider",
                        style: if !is_list { "color: #00ffaa; background: rgba(255, 255, 255, 0.06); border-radius: 3px;" } else { "color: #a0a8a4; background: transparent;" },
                        onclick: move |_| view_mode.set(ViewMode::Map),
                        "Map"
                    }
                }
            }

            // Content area
            if !is_list {
                MapView {
                    startups,
                    similarities,
                    sort_by_similarity: sort_mode() == SortMode::Similarity,
                    has_search,
                    min_team_size_filter: min_team_size_filter(),
                    min_similarity_filter: min_similarity_filter(),
                    zoom,
                    offset_x,
                    offset_y,
                    target_zoom,
                    target_offset_x,
                    target_offset_y,
                    is_dragging,
                    last_mouse_x,
                    last_mouse_y,
                }
            } else if !has_search && is_sim_sort {
                div {
                    class: "flex-1 flex flex-col items-center justify-center gap-3",
                    p {
                        class: "text-sm m-0 uppercase tracking-widest",
                        style: "color: #808884;",
                        "Awaiting query input"
                    }
                    p {
                        class: "text-xs m-0",
                        style: "color: #707870;",
                        "5,000+ entities indexed // vector similarity search"
                    }
                }
            } else if is_searching() {
                div {
                    class: "flex-1 flex items-center justify-center text-xs uppercase tracking-widest",
                    style: "color: #00ffaa;",
                    "Processing query..."
                }
            } else {
                {
                    let sims = similarities();
                    let all_startups = startups();
                    let mut indexed: Vec<(usize, f32)> = sims.iter().copied().enumerate().collect();

                    match sort_mode() {
                        SortMode::Similarity => {
                            indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                            let min_ts = min_team_size_filter();
                            indexed.retain(|(idx, _)| all_startups[*idx].team_size >= min_ts);
                        }
                        SortMode::TeamSize => {
                            indexed.sort_by(|a, b| {
                                all_startups[b.0].team_size.cmp(&all_startups[a.0].team_size)
                            });
                            let min_sim = min_similarity_filter();
                            if has_search {
                                indexed.retain(|(_, sim)| *sim >= min_sim);
                            }
                        }
                    }

                    let results: Vec<(usize, f32)> = indexed.into_iter().take(50).collect();
                    let result_count = results.len();

                    rsx! {
                        div {
                            class: "flex-1 overflow-y-auto p-6",

                            p {
                                class: "text-xs mb-4 uppercase tracking-wider",
                                style: "color: #a0a8a4;",
                                "{result_count} results"
                                if has_search {
                                    " // query: \"{search}\""
                                }
                            }

                            // Column headers
                            div {
                                class: "flex items-center gap-3 px-3 pb-2 mb-1",
                                style: "border-bottom: 1px solid rgba(255, 255, 255, 0.08);",
                                span {
                                    class: "text-xs uppercase tracking-wider w-8 text-right shrink-0",
                                    style: "color: #606860;",
                                    "#"
                                }
                                span {
                                    class: "text-xs uppercase tracking-wider shrink-0",
                                    style: "color: #606860; width: 36px;",
                                }
                                span {
                                    class: "text-xs uppercase tracking-wider flex-1",
                                    style: "color: #606860;",
                                    "Name"
                                }
                                span {
                                    class: "text-xs uppercase tracking-wider flex-1 hidden md:block",
                                    style: "color: #606860;",
                                    "Tagline"
                                }
                                span {
                                    class: "text-xs uppercase tracking-wider w-16 text-right shrink-0",
                                    style: if !is_sim_sort { "color: #00ffaa;" } else { "color: #606860;" },
                                    "Team"
                                }
                                if has_search {
                                    span {
                                        class: "text-xs uppercase tracking-wider w-16 text-right shrink-0",
                                        style: if is_sim_sort { "color: #00ffaa;" } else { "color: #606860;" },
                                        "Match"
                                    }
                                }
                            }

                            // Results list
                            div {
                                class: "flex flex-col",
                                for (rank, (idx, sim)) in results.into_iter().enumerate() {
                                    StartupRow {
                                        startup: all_startups[idx].clone(),
                                        similarity: sim,
                                        show_similarity: has_search,
                                        sort_mode: sort_mode(),
                                        rank: rank + 1,
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[component]
fn StartupRow(startup: StartupWithPos, similarity: f32, show_similarity: bool, sort_mode: SortMode, rank: usize) -> Element {
    let match_pct = (similarity * 100.0).round() as u32;
    let is_team_sort = sort_mode == SortMode::TeamSize;

    rsx! {
        div {
            class: "flex items-center gap-3 px-3 py-2",
            style: "border-bottom: 1px solid rgba(255, 255, 255, 0.04);",

            // Rank
            span {
                class: "text-xs w-8 text-right shrink-0",
                style: "color: #505850;",
                "{rank}"
            }

            // Logo
            img {
                src: "{startup.logo_url}",
                class: "w-7 h-7 object-cover shrink-0",
                style: "border-radius: 3px;",
                alt: "{startup.name}"
            }

            // Name (clickable link)
            a {
                href: "{startup.link}",
                target: "_blank",
                class: "text-xs font-medium truncate flex-1 no-underline transition-colors",
                style: "color: #e0e4e2;",
                "{startup.name}"
            }

            // Tagline
            span {
                class: "text-xs truncate flex-1 hidden md:block",
                style: "color: #707870;",
                "{startup.tagline}"
            }

            // Team size
            span {
                class: "text-xs w-16 text-right shrink-0",
                style: if is_team_sort { "color: #00ffaa;" } else { "color: #a0a8a4;" },
                "{startup.team_size}"
            }

            // Similarity
            if show_similarity {
                span {
                    class: "text-xs font-medium w-16 text-right shrink-0",
                    style: if !is_team_sort { "color: #00ffaa;" } else { "color: #a0a8a4;" },
                    "{match_pct}%"
                }
            }
        }
    }
}
