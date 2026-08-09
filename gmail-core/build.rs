//! Build script for gmail-core:
//! 1. Trains zstd dictionaries for Gmail API request/response compression
//! 2. Generates compile-time constants

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Deterministic FNV-1a hash of byte slices (no random seed, stable across runs).
fn hash_samples(samples: &[Vec<u8>]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for s in samples {
        hash ^= s.len() as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        for &b in s {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

/// Check if cached dict and hash file match the current sample hash.
async fn is_cached(dict_path: &Path, expected_hash: u64) -> bool {
    if !dict_path.exists() {
        return false;
    }
    let hash_path = dict_path.with_extension("zdict.hash");
    let Ok(stored) = fs::read_to_string(&hash_path).await else {
        return false;
    };
    stored.trim() == expected_hash.to_string()
}

/// Train a zstd dictionary from samples, writing both the dict and hash file.
async fn train_dict(name: &str, samples: Vec<Vec<u8>>, dict_dir: &Path) -> Result<()> {
    let dict_path = dict_dir.join(format!("{name}.zdict"));
    let hash_path = dict_path.with_extension("zdict.hash");

    if samples.is_empty() {
        println!(
            "cargo:warning=Skipped {name} dictionary training: no usable samples; keeping {}",
            dict_path.display()
        );
        return Ok(());
    }

    let hash = hash_samples(&samples);
    if is_cached(&dict_path, hash).await {
        println!(
            "cargo:warning=Skipped {name} dictionary training: cached dictionary matches {} samples",
            samples.len()
        );
        return Ok(());
    }

    let refs: Vec<&[u8]> = samples.iter().map(Vec::as_slice).collect();
    let dict = zstd::dict::from_samples(&refs, 16384)
        .with_context(|| format!("failed to train {name} dictionary"))?;

    fs::write(&dict_path, &dict)
        .await
        .with_context(|| format!("failed to write {name}.zdict"))?;
    fs::write(&hash_path, hash.to_string())
        .await
        .with_context(|| format!("failed to write {name}.zdict.hash"))?;

    println!(
        "cargo:warning=Trained {name} dictionary: {} bytes from {} samples",
        dict.len(),
        samples.len()
    );

    Ok(())
}

fn usable_text_sample(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    (!trimmed.is_empty() && !trimmed.starts_with('#')).then_some(trimmed)
}

async fn load_samples(path: &Path) -> Result<Vec<Vec<u8>>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;

    let samples: Vec<Vec<u8>> = content
        .lines()
        .filter_map(usable_text_sample)
        .map(|line| line.as_bytes().to_vec())
        .collect();

    Ok(samples)
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=assets/dict/samples");
    println!("cargo:rerun-if-changed=src/schemas");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").context("OUT_DIR not set")?);

    // Create dict directory
    let dict_dir = PathBuf::from("assets/dict");
    fs::create_dir_all(&dict_dir).await?;

    // Train dictionaries for different payload types
    let samples_dir = PathBuf::from("assets/dict/samples");
    
    // Train search request/response dictionary
    let search_samples = load_samples(&samples_dir.join("search.txt")).await?;
    train_dict("search", search_samples.clone(), &dict_dir).await?;

    // Train message dictionary
    let message_samples = load_samples(&samples_dir.join("message.txt")).await?;
    train_dict("message", message_samples.clone(), &dict_dir).await?;

    // Train batch request dictionary
    let batch_samples = load_samples(&samples_dir.join("batch.txt")).await?;
    train_dict("batch", batch_samples.clone(), &dict_dir).await?;

    // Unified dictionary for all Gmail API traffic
    let mut all_samples = Vec::new();
    all_samples.extend(search_samples);
    all_samples.extend(message_samples);
    all_samples.extend(batch_samples);
    train_dict("gmail-unified", all_samples, &dict_dir).await?;

    // Copy dicts to OUT_DIR for runtime access
    let mut entries = fs::read_dir(&dict_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.path().extension().is_some_and(|e| e == "zdict") {
            let dest = out_dir.join(entry.file_name());
            fs::copy(entry.path(), &dest).await?;
            println!("cargo:rustc-env={}={}", entry.file_name().to_string_lossy().to_uppercase().replace(".", "_"), dest.display());
        }
    }

    Ok(())
}