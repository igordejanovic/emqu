use clap::{Parser, Subcommand};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use glob::glob;
use indicatif::{ProgressBar, ProgressFinish, ProgressStyle};
use serde_json::{from_reader, to_writer};
use std::{ffi::OsStr, fs, path::PathBuf};
use text_splitter::{ChunkConfig, TextSplitter};
use tokenizers::Tokenizer;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

/// Chunk, embed and query textual files.
#[derive(Subcommand)]
enum Command {
    /// Chunk files into semantically sensible pieces
    Chunk {
        /// Glob pattern for files to process
        pattern: String,
        /// Output folder for chunks
        output: PathBuf,
    },
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

    fn get_progress(len: u64, message: &'static str) -> ProgressBar {
        ProgressBar::new(len)
            .with_finish(ProgressFinish::AndLeave)
            .with_style(
                ProgressStyle::with_template(
                    "{msg}: {wide_bar} [{human_pos} / {human_len} | {percent}%]",
                )
                .unwrap(),
            )
            .with_message(message)
    }

    match args.command {
        Command::Chunk { pattern, output } => {
            fs::create_dir_all(&output)?;
            let files_to_chunk: Vec<Result<PathBuf, glob::GlobError>> = glob(&pattern)?.collect();
            println!("Chunking {} document(s).", files_to_chunk.len());

            let tokenizer = Tokenizer::from_pretrained("bert-base-cased", None).unwrap();
            let max_tokens = 1000;
            let splitter = TextSplitter::new(ChunkConfig::new(max_tokens).with_sizer(tokenizer));

            let progress = get_progress(files_to_chunk.len() as u64, "Chunking");

            for entry in files_to_chunk {
                let path = entry?;
                let content = fs::read_to_string(&path)?;
                let base_name = path
                    .file_stem()
                    .unwrap_or(OsStr::new("unknown"))
                    .to_string_lossy();
                let extension = path
                    .extension()
                    .unwrap_or(OsStr::new("txt"))
                    .to_string_lossy();

                let chunks = splitter.chunks(&content).collect::<Vec<_>>();
                let line_counts: Vec<(usize, usize)> = chunks
                    .iter()
                    .scan(0, |acc, chunk| {
                        let lines = chunk.lines().count();
                        let start = *acc + 1;
                        *acc += lines;
                        Some((start, *acc))
                    })
                    .collect();

                for (i, (chunk, (start, end))) in chunks.iter().zip(line_counts).enumerate() {
                    let header = format!("From {}, lines {} - {}\n\n", base_name, start, end);
                    let chunk_file = output.join(format!("{}-{}.{}", base_name, i + 1, extension));
                    fs::write(chunk_file, header + chunk)?;
                }
                progress.inc(1u64);
            }

            println!("Successfully chunked documents into {}", output.display());
        }
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
