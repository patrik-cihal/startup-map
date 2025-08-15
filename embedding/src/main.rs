use std::f32;

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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct StartupWithPos {
    name: String,
    tagline: String,
    pos_x: f32,
    pos_y: f32,
}

fn main() {
    let startups = csv::Reader::from_path("yc_company_details.csv")
        .unwrap()
        .deserialize::<Startup>()
        .map(|res| res.unwrap())
        .collect::<Vec<_>>();

    let mut model = TextEmbedding::try_new(Default::default()).unwrap();

    let embeddings = model
        .embed(startups.iter().map(|x| &x.tagline).collect(), None)
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

    let mut startups = startups
        .into_iter()
        .zip(embeddings)
        .map(|(s, pos)| StartupWithPos {
            name: s.name,
            tagline: s.tagline,
            pos_x: pos.0,
            pos_y: pos.1,
        })
        .collect::<Vec<_>>();

    let mut wtr = Writer::from_path("startups.csv").unwrap();

    for startup in startups {
        wtr.serialize(startup).unwrap();
    }
    wtr.flush().unwrap();
}
