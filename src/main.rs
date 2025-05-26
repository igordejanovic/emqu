use clap::{Parser, Subcommand};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use glob::glob;
use serde_json::{from_reader, to_writer};
use std::{ffi::OsStr, fs, path::PathBuf};

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate embeddings from files
    Embed {
        /// Glob pattern for files to process
        pattern: String,
        /// Output JSON file for embeddings
        output: PathBuf,
    },
    /// Query similar documents
    Query {
        /// Input JSON file with embeddings
        input: PathBuf,
        /// Query text
        query: String,

        #[arg(short, long, default_value_t = 1)]
        top_k: usize,
    },
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot_product / (norm_a * norm_b)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2)
            .with_show_download_progress(true)
            .with_cache_dir(
                dirs::cache_dir()
                    .expect("Could not get cache dir")
                    .join("emqu-models"),
            ),
    )?;

    match args.command {
        Command::Embed { pattern, output } => {
            let mut documents = Vec::new();

            let embed_files: Vec<_> = glob(&pattern)?.collect();
            println!("Embedding {} document(s).", embed_files.len());

            for entry in embed_files {
                let path = entry?;
                let base_name = path.file_name().unwrap_or(OsStr::new("unknown"));
                let content = fs::read_to_string(&path)?;
                documents.push(format!("From: {base_name:?}\n{content}"));
            }

            let embeddings = model.embed(documents.clone(), None)?;
            let output_data: Vec<(String, Vec<f32>)> =
                documents.into_iter().zip(embeddings).collect();

            let file = fs::File::create(output)?;
            to_writer(file, &output_data)?;

            println!(
                "Successfully generated embeddings for {} documents",
                output_data.len()
            );
        }
        Command::Query {
            input,
            query,
            top_k,
        } => {
            let file = fs::File::open(input)?;
            let stored_embeddings: Vec<(String, Vec<f32>)> = from_reader(file)?;

            let query_embedding = &model.embed(vec![query], None)?[0];

            let mut results: Vec<(f32, String)> = stored_embeddings
                .into_iter()
                .map(|(doc, embedding)| {
                    let score = cosine_similarity(query_embedding, &embedding);
                    (score, doc)
                })
                .collect();

            results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

            let top_results = results.into_iter().take(top_k).collect::<Vec<_>>();

            for (_score, doc) in top_results {
                println!("{}\n\n", doc.trim());
            }
        }
    }
    Ok(())
}
