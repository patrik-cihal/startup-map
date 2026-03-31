use std::sync::Arc;
use std::time::Instant;

use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
};
use fastembed::TextEmbedding;
use futures::stream::{self, StreamExt};
use ndarray::Array2;
use pacmap::{Configuration, fit_transform};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Startup {
    company_link: String,
    name: String,
    tagline: String,
    logo_url: String,
    founded: Option<u32>,
    team_size: Option<u32>,
    long_description: String,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    batch: Option<String>,
}

impl Startup {
    /// Return founded year, falling back to parsing the year from the batch field (e.g. "Summer 2021").
    fn founded_year(&self) -> u32 {
        if let Some(y) = self.founded {
            if y > 0 {
                return y;
            }
        }
        if let Some(batch) = &self.batch {
            for part in batch.split_whitespace() {
                if let Ok(y) = part.parse::<u32>() {
                    return y;
                }
            }
        }
        0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct StartupWithPos {
    link: String,
    name: String,
    tagline: String,
    pos_x: f32,
    pos_y: f32,
    team_size: u32,
    #[serde(default)]
    founded: u32,
    #[serde(default = "default_status")]
    status: String,
    logo_url: String,
    embedding: Vec<f32>,
}

fn default_status() -> String {
    "Active".to_string()
}

const SYSTEM_PROMPT: &str = "You are an expert at writing clear, consistent startup taglines. Your task is to normalize startup taglines into a standard format that clearly describes what the company does, for whom, and how.

Rules for normalized taglines:
1. Start with the core action/service the company provides
2. Specify the target market/industry
3. Mention the key technology/method if relevant
4. Keep it concise (5-12 words)
5. Use consistent formatting and capitalization
6. Focus on the value proposition, not marketing fluff

Examples:
- Input: 'Foundational Voice AI for underserved languages' → Output: 'Voice AI models for underserved languages'
- Input: 'AI agent that does QA on mobile apps' → Output: 'Automated mobile app testing using AI'
- Input: 'RealRoots is a mobile app that guarantees women lifelong friendships' → Output: 'Friendship matching platform for women'

Respond with ONLY the normalized tagline, no explanation.";

const CONCURRENCY: usize = 20;

async fn map_tagline(
    startup: &Startup,
    client: &Client<OpenAIConfig>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let user_prompt = format!(
        "Company: {}\nCurrent tagline: {}\nDescription: {}\n\nNormalize this tagline:",
        startup.name,
        startup.tagline,
        startup.long_description.chars().take(500).collect::<String>()
    );

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-5-mini")
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(SYSTEM_PROMPT)
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_prompt)
                .build()?
                .into(),
        ])
        .build()?;

    let response = client.chat().create(request).await?;
    Ok(response.choices[0].clone().message.content.unwrap())
}

fn format_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.0}s", secs)
    } else {
        format!("{}m{:02.0}s", secs as u64 / 60, secs % 60.0)
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();
    let total_start = Instant::now();

    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("--patch") {
        patch_metadata();
        return;
    }

    let startups = csv::Reader::from_path("../scraping/yc_company_details.csv")
        .unwrap()
        .deserialize::<Startup>()
        .map(|res| res.unwrap())
        .collect::<Vec<_>>();

    let total = startups.len();
    println!("[1/4] Loaded {total} startups");

    // Load cached taglines
    let cached_taglines = std::fs::read_to_string("cached_taglines.txt")
        .unwrap_or_default()
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<String>>();

    let cached_count = cached_taglines.len().min(total);
    let new_count = total.saturating_sub(cached_count);
    println!("[2/4] Normalizing taglines: {cached_count} cached, {new_count} new");

    // Use cached taglines for existing startups
    let mut normalized_startups: Vec<Startup> = startups[..cached_count]
        .iter()
        .cloned()
        .zip(cached_taglines[..cached_count].iter())
        .map(|(mut s, tagline)| {
            s.tagline = tagline.clone();
            s
        })
        .collect();

    // Process new startups in parallel
    if new_count > 0 {
        let client = Client::new();
        let new_startups = &startups[cached_count..];
        let completed = Arc::new(Mutex::new(0usize));
        let start = Instant::now();

        let results: Vec<(usize, Result<String, _>)> = stream::iter(new_startups.iter().enumerate())
            .map(|(i, startup)| {
                let client = &client;
                let completed = completed.clone();
                async move {
                    let mut retries = 10;
                    let result = loop {
                        match map_tagline(startup, client).await {
                            Ok(tagline) => break Ok(tagline),
                            Err(e) => {
                                retries -= 1;
                                if retries == 0 {
                                    break Err(e);
                                }
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            }
                        }
                    };

                    let mut done = completed.lock().await;
                    *done += 1;
                    let elapsed = start.elapsed().as_secs_f64();
                    let rate = *done as f64 / elapsed;
                    let remaining = (new_count - *done) as f64 / rate;
                    match &result {
                        Ok(tagline) => println!(
                            "  [{}/{}] {} - \"{}\" (ETA: {})",
                            *done, new_count, startup.name, tagline, format_duration(remaining)
                        ),
                        Err(e) => println!(
                            "  [{}/{}] {} - FAILED: {}",
                            *done, new_count, startup.name, e
                        ),
                    }

                    (i, result)
                }
            })
            .buffer_unordered(CONCURRENCY)
            .collect()
            .await;

        // Sort by original index and collect taglines
        let mut sorted_results: Vec<(usize, Result<String, _>)> = results.into_iter().collect();
        sorted_results.sort_by_key(|(i, _)| *i);

        let mut new_taglines = Vec::new();
        for (_, result) in sorted_results {
            match result {
                Ok(tagline) => {
                    new_taglines.push(tagline);
                }
                Err(e) => {
                    eprintln!("Fatal: failed to normalize tagline: {e}");
                    return;
                }
            }
        }

        // Update cache
        let mut all_taglines: Vec<String> = cached_taglines[..cached_count].to_vec();
        all_taglines.extend(new_taglines.iter().cloned());
        std::fs::write("cached_taglines.txt", all_taglines.join("\n")).unwrap();

        // Build normalized startups for new entries
        for (startup, tagline) in new_startups.iter().zip(new_taglines) {
            let mut s = startup.clone();
            s.tagline = tagline;
            normalized_startups.push(s);
        }

        println!("  Done in {}", format_duration(start.elapsed().as_secs_f64()));
    }

    println!("[3/4] Generating embeddings...");
    let embed_start = Instant::now();
    let mut model = TextEmbedding::try_new(Default::default()).unwrap();

    let taglines: Vec<String> = normalized_startups.iter().map(|x| x.tagline.clone()).collect();
    let embeddings = model.embed(taglines, None).unwrap();

    let embeddings = Array2::from_shape_vec(
        (embeddings.len(), embeddings[0].len()),
        embeddings.into_iter().flatten().collect(),
    )
    .unwrap();

    let high_dim_embeddings = embeddings.clone();
    println!("  Embeddings done in {}", format_duration(embed_start.elapsed().as_secs_f64()));

    println!("[4/4] Reducing dimensions (PaCMAP)...");
    let pacmap_start = Instant::now();
    let config = Configuration::builder().embedding_dimensions(2).build();
    let (embeddings, _) = fit_transform(embeddings.view(), config).unwrap();

    let mut min_val = f32::MAX;
    let mut max_val = f32::MIN;
    for embedding in &embeddings {
        min_val = min_val.min(*embedding);
        max_val = max_val.max(*embedding);
    }
    let range = max_val - min_val;

    let embeddings = embeddings
        .outer_iter()
        .map(|row| ((row[0] - min_val) / range, (row[1] - min_val) / range))
        .collect::<Vec<_>>();
    println!("  PaCMAP done in {}", format_duration(pacmap_start.elapsed().as_secs_f64()));

    let startups = normalized_startups
        .into_iter()
        .zip(high_dim_embeddings.outer_iter())
        .zip(embeddings)
        .map(|((s, emb), pos)| {
            let founded = s.founded_year();
            StartupWithPos {
            link: s.company_link,
            name: s.name,
            tagline: s.tagline,
            pos_x: pos.0,
            pos_y: pos.1,
            team_size: s.team_size.unwrap_or(0),
            founded,
            status: s.status.unwrap_or_else(|| "Active".to_string()),
            logo_url: s.logo_url.split('?').next().unwrap_or(&s.logo_url).to_string(),
            embedding: emb.to_vec(),
        }})
        .collect::<Vec<_>>();

    let json = serde_json::to_string_pretty(&startups).unwrap();
    std::fs::write("startups.json", json).unwrap();

    println!(
        "Done! Wrote {} startups to startups.json (total: {})",
        startups.len(),
        format_duration(total_start.elapsed().as_secs_f64())
    );
}

fn patch_metadata() {
    println!("Patching startups.json with metadata from CSV...");

    let csv_startups: Vec<Startup> = csv::Reader::from_path("../scraping/yc_company_details.csv")
        .unwrap()
        .deserialize::<Startup>()
        .map(|res| res.unwrap())
        .collect();

    // Build lookup by link
    let mut csv_map: std::collections::HashMap<String, &Startup> = std::collections::HashMap::new();
    for s in &csv_startups {
        csv_map.insert(s.company_link.clone(), s);
    }

    let json_str = std::fs::read_to_string("startups.json").unwrap();
    let mut startups: Vec<StartupWithPos> = serde_json::from_str(&json_str).unwrap();

    let mut updated = 0;
    for s in &mut startups {
        if let Some(csv) = csv_map.get(&s.link) {
            s.founded = csv.founded_year();
            s.status = csv.status.clone().unwrap_or_else(|| "Active".to_string());
            s.logo_url = csv.logo_url.split('?').next().unwrap_or(&csv.logo_url).to_string();
            updated += 1;
        }
    }

    let json = serde_json::to_string_pretty(&startups).unwrap();
    std::fs::write("startups.json", json).unwrap();
    println!("Patched {updated}/{} startups", startups.len());
}
