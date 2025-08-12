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

    let config = Configuration::default();

    let (embeddings, _) = fit_transform(embeddings.view(), config).unwrap();

    for (i, embedding) in embeddings.into_iter().enumerate() {
        println!("{i} {embedding:?}");
    }
}
