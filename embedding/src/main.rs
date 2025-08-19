use std::time::Duration;

use async_openai::{
    Client,
    config::{Config, OpenAIConfig},
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
};
use csv::Writer;
use fastembed::TextEmbedding;
use ndarray::Array2;
use pacmap::{Configuration, fit_transform};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Startup {
    company_link: String,
    name: String,
    tagline: String,
    logo_url: String,
    founded: Option<u32>,
    team_size: Option<u32>,
    long_description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct StartupWithPos {
    link: String,
    name: String,
    tagline: String,
    pos_x: f32,
    pos_y: f32,
    team_size: u32,
    logo_url: String,
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

async fn map_tagline(
    startup: &Startup,
    client: &Client<OpenAIConfig>,
) -> Result<String, Box<dyn std::error::Error>> {
    let user_prompt = format!(
        "Company: {}\nCurrent tagline: {}\nDescription: {}\n\nNormalize this tagline:",
        startup.name,
        startup.tagline,
        startup
            .long_description
            .chars()
            .take(500)
            .collect::<String>()
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

#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();

    let startups = csv::Reader::from_path("../scraping/yc_company_details.csv")
        .unwrap()
        .deserialize::<Startup>()
        .map(|res| res.unwrap())
        .collect::<Vec<_>>();

    println!("Normalizing taglines for {} startups...", startups.len());

    // Normalize taglines using LLM or fallback
    // Load existing cached taglines
    let mut cached_taglines = std::fs::read_to_string("cached_taglines.txt")
        .unwrap_or_default()
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<String>>();

    let mut normalized_startups = Vec::new();
    let mut new_taglines = Vec::new();

    let client = Client::new();

    for (i, startup) in startups.iter().enumerate() {
        if i % 10 == 0 {
            println!("Processed {}/{} startups", i, startups.len());
        }

        let normalized_tagline = if i < cached_taglines.len() {
            cached_taglines[i].clone()
        } else {
            let mut mx_retries = 10;
            let tagline = loop {
                if let Ok(tagline) = map_tagline(startup, &client).await {
                    break tagline;
                }
                mx_retries -= 1;
                if mx_retries == 0 {
                    println!("Max retries exceeded for startup {}", startup.name);
                    return;
                }
            };
            new_taglines.push(tagline.clone());
            tagline
        };

        println!("{normalized_tagline}");

        let mut normalized_startup = startup.clone();
        normalized_startup.tagline = normalized_tagline;
        normalized_startups.push(normalized_startup);

        // Save cache every 10 items
        if i % 10 == 9 && !new_taglines.is_empty() {
            cached_taglines.extend(new_taglines.clone());
            std::fs::write("cached_taglines.txt", cached_taglines.join("\n")).unwrap();
            new_taglines.clear();
        }
    }

    // Save any remaining new taglines
    if !new_taglines.is_empty() {
        cached_taglines.extend(new_taglines);
        std::fs::write("cached_taglines.txt", cached_taglines.join("\n")).unwrap();
    }

    println!("Generating embeddings...");
    let mut model = TextEmbedding::try_new(Default::default()).unwrap();

    let embeddings = model
        .embed(
            normalized_startups.iter().map(|x| &x.tagline).collect(),
            None,
        )
        .unwrap();

    let embeddings = Array2::from_shape_vec(
        (embeddings.len(), embeddings[0].len()),
        embeddings.into_iter().flatten().collect(),
    )
    .unwrap();

    let config = Configuration::builder().embedding_dimensions(2).build();

    let (embeddings, _) = fit_transform(embeddings.view(), config).unwrap();
    let mut min_val = f32::MAX;
    let mut max_val = f32::MIN;
    for embedding in &embeddings {
        min_val = min_val.min(*embedding);
        max_val = max_val.max(*embedding);
    }

    let range = max_val - min_val;

    // normalize embeddings from 0 to 1
    let embeddings = embeddings
        .outer_iter()
        .map(|row| ((row[0] - min_val) / range, (row[1] - min_val) / range))
        .collect::<Vec<_>>();

    let startups = normalized_startups
        .into_iter()
        .zip(embeddings)
        .map(|(s, pos)| StartupWithPos {
            link: s.company_link,
            name: s.name,
            tagline: s.tagline,
            pos_x: pos.0,
            pos_y: pos.1,
            team_size: s.team_size.unwrap_or(0),
            logo_url: s
                .logo_url
                .split('?')
                .next()
                .unwrap_or(&s.logo_url)
                .to_string(),
        })
        .collect::<Vec<_>>();

    let mut wtr = Writer::from_path("startups.csv").unwrap();

    for startup in startups {
        wtr.serialize(startup).unwrap();
    }
    wtr.flush().unwrap();
}
