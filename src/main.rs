use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use rayon::prelude::*;
use regex::Regex;
use std::{
    fs::{self, File},
    io::{self, BufRead, prelude::*},
    path::{Path, PathBuf},
    sync::Arc,
};

/// A simple text processor for the command line.
#[derive(Parser)]
#[command(
    name = "texpro",
    version,
    about = "A simple text processor for the command line.",
    long_about = "A versatile text processing tool for the command line that can search for regex patterns in single files or entire directories, plus basic text edition features."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for regex patterns in a file.
    Search {
        /// Path to the target file.
        file_path: PathBuf,
        /// Regex pattern to search for.
        pattern: String,
    },
    /// Compare two text files and report differing line numbers.
    Compare {
        /// Path to the first file.
        file_1_path: PathBuf,
        /// Path to the second file.
        file_2_path: PathBuf,
    },
    /// Search for regex patterns in all files within a directory.
    Directory {
        /// Path to the directory.
        dir_path: PathBuf,
        /// Regex pattern to search for.
        pattern: String,
    },
    /// Extract text from a file based on a regex pattern.
    Extract {
        /// Path to the target file.
        file_path: PathBuf,
        /// Regex pattern for extraction.
        pattern: String,
    },
    /// Replace occurrences of a regex pattern with a specified string.
    Replace {
        /// Path to the target file.
        file_path: PathBuf,
        /// Regex pattern to replace.
        pattern: String,
        /// The replacement text.
        replacement: String,
    },
    /// Display word, character, and byte statistics for a file.
    Stats {
        /// Path to the target file.
        file_path: PathBuf,
    },
    /// Format text according to specified options.
    Format {
        /// Path to the target file.
        file_path: PathBuf,
        /// The format option.
        format_option: String,
    },
    /// Validate file lines against a regex pattern.
    Validate {
        /// Path to the target file.
        file_path: PathBuf,
        /// Regex pattern for validation.
        validation_pattern: String,
    },
}

fn validate_input(file_path: &Path, expect_file: bool) -> Result<()> {
    let metadata =
        fs::metadata(file_path).with_context(|| format!("Cannot access '{:?}'", file_path))?;

    if expect_file && !metadata.is_file() {
        return Err(anyhow!("'{:?}' is not a regular file", file_path));
    }
    if !expect_file && !metadata.is_dir() {
        return Err(anyhow!("'{:?}' is not a directory", file_path));
    }
    Ok(())
}

fn is_plain_text(file_path: &Path) -> bool {
    let mut file = match File::open(file_path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let mut buffer = [0u8; 1024];
    let n = file.read(&mut buffer).unwrap_or(0);
    if n == 0 {
        return true;
    }

    let bytes = &buffer[..n];

    for &b in bytes {
        if b == 0 {
            return false;
        }
        if b < 7 || (b > 13 && b < 32) {
            return false;
        }
    }
    true
}

fn read_file(file_path: &Path) -> Result<Vec<String>> {
    if !is_plain_text(file_path) {
        return Err(anyhow!("'{:?}' is not a plain text file", file_path));
    }
    let file = File::open(file_path).context("Failed to open file")?;
    let buf_reader = io::BufReader::new(file);
    let lines = buf_reader.lines().collect::<Result<Vec<_>, _>>()?;
    Ok(lines)
}

fn search_patterns(lines: &[String], regex_pattern: &Regex) {
    let mut match_count: usize = 0;
    let mut matching_lines = Vec::new();

    for (line_index, line_content) in lines.iter().enumerate() {
        if regex_pattern.find(line_content).is_some() {
            match_count += 1;
            matching_lines.push((line_index + 1).to_string());
        }
    }

    if match_count > 0 {
        println!("Found {} matches at lines:", match_count);
        for line_number in matching_lines {
            println!("  {}", line_number);
        }
    } else {
        println!("No matches found.");
    }
}

fn compare_files(file_1_path: &Path, file_2_path: &Path) -> Result<()> {
    let file_1_lines = read_file(file_1_path)?;
    let file_2_lines = read_file(file_2_path)?;

    let mut file_1_iter = file_1_lines.iter();
    let mut file_2_iter = file_2_lines.iter();
    let mut current_line_number: usize = 1;
    let mut diff_count: usize = 0;
    let mut diff_lines = Vec::new();

    loop {
        match (file_1_iter.next(), file_2_iter.next()) {
            (Some(line_1), Some(line_2)) if line_1 != line_2 => {
                diff_lines.push(current_line_number.to_string());
                diff_count += 1;
                current_line_number += 1;
            }
            (Some(_line_1), Some(_line_2)) => {
                current_line_number += 1;
            }
            (Some(_), None) | (None, Some(_)) => {
                diff_lines.push(current_line_number.to_string());
                diff_count += 1;
                break;
            }
            (None, None) => break,
        }
    }

    if !diff_lines.is_empty() {
        println!("Differences found at lines:");
        for line_number in diff_lines {
            println!("  {}", line_number);
        }
    }

    let total_lines = file_1_lines.len().max(file_2_lines.len());
    let diff_percent = if total_lines > 0 {
        (diff_count as f64 / total_lines as f64 * 100.0) as usize
    } else {
        0
    };

    println!(
        "Comparison complete. {} differences found, files differ by {}%",
        diff_count, diff_percent
    );
    Ok(())
}

fn search_in_directory(dir_path: &Path, regex_pattern: Arc<Regex>) -> Result<()> {
    let entries = fs::read_dir(dir_path)
        .with_context(|| format!("Error reading directory {:?}", dir_path))?;

    let file_paths: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .collect();

    let results: Vec<_> = file_paths
        .par_iter()
        .filter_map(|entry| {
            let entry_path = entry.path();
            let file_name = entry_path.file_name()?.to_str()?.to_string();

            let file_lines = read_file(&entry_path).ok()?;
            let mut matching_line_numbers = Vec::new();

            for (line_index, line_content) in file_lines.iter().enumerate() {
                if regex_pattern.find(line_content).is_some() {
                    matching_line_numbers.push((line_index + 1).to_string());
                }
            }

            if matching_line_numbers.is_empty() {
                None
            } else {
                Some((file_name, matching_line_numbers))
            }
        })
        .collect();

    for (file_name, matching_line_numbers) in results {
        println!("\nFile {}", file_name);
        for line_number in matching_line_numbers {
            println!("Line {}", line_number);
        }
    }

    println!(
        "\nDirectory search complete. Processed {} files.",
        file_paths.len()
    );
    Ok(())
}

fn extract_text(file_lines: &[String], regex_pattern: &Regex) {
    let mut match_count: usize = 0;
    for line_content in file_lines {
        if let Some(matched_text_span) = regex_pattern.find(line_content) {
            println!(
                "{}",
                &line_content[matched_text_span.start()..matched_text_span.end()]
            );
            match_count += 1;
        }
    }
    if match_count == 0 {
        println!("No matches found.");
    }
}

fn replace_text(
    file_lines: &[String],
    file_path: &Path,
    regex_pattern: &Regex,
    replacement: &str,
) -> Result<()> {
    let mut temp_file_path = file_path.to_path_buf();
    temp_file_path.set_extension("tmp");

    let mut temp_file = File::create(&temp_file_path).context("Error creating temporary file")?;

    let mut replacement_count: usize = 0;
    for original_line in file_lines {
        let modified_line = regex_pattern
            .replace_all(original_line, replacement)
            .to_string();
        if modified_line != *original_line {
            replacement_count += 1;
        }
        writeln!(temp_file, "{}", modified_line)?;
    }
    drop(temp_file);

    fs::rename(&temp_file_path, file_path)
        .inspect_err(|_e| {
            let _ = fs::remove_file(&temp_file_path);
        })
        .context("Error finalizing replacement")?;

    println!("File modified in place: {:?}", file_path);
    println!("{} replacements made.", replacement_count);
    Ok(())
}

fn print_text_stats(file_lines: &[String]) {
    let mut word_count = 0;
    let mut char_count = 0;
    let mut byte_count = 0;

    for line in file_lines {
        word_count += line.split_whitespace().count();
        char_count += line.chars().count();
        byte_count += line.len();
    }

    println!("Word count {}", word_count);
    println!("Character count {}", char_count);
    println!("Byte count {}", byte_count);
}

fn format_text(file_lines: &[String], format_option: &str) {
    match format_option {
        "uppercase" => {
            for line in file_lines {
                println!("{}", line.to_uppercase());
            }
        }
        "lowercase" => {
            for line in file_lines {
                println!("{}", line.to_lowercase());
            }
        }
        _ => eprintln!("Unsupported format option. Use uppercase or lowercase."),
    }
}

fn validate_text(file_lines: &[String], regex_pattern: &Regex) {
    let mut passed = 0;
    let mut failed = 0;

    for line in file_lines {
        if regex_pattern.is_match(line) {
            println!("Validation successful for line {}", line);
            passed += 1;
        } else {
            println!("Validation failed for line {}", line);
            failed += 1;
        }
    }
    println!("Validation complete: {} passed, {} failed", passed, failed);
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Search { file_path, pattern } => {
            let regex = Regex::new(&pattern)
                .with_context(|| format!("Invalid regex pattern {}", pattern))?;
            validate_input(&file_path, true)?;
            let lines = read_file(&file_path)?;
            search_patterns(&lines, &regex);
        }
        Commands::Compare {
            file_1_path,
            file_2_path,
        } => {
            validate_input(&file_1_path, true)?;
            validate_input(&file_2_path, true)?;
            compare_files(&file_1_path, &file_2_path)?;
        }
        Commands::Directory { dir_path, pattern } => {
            let regex = Regex::new(&pattern)
                .with_context(|| format!("Invalid regex pattern {}", pattern))?;
            validate_input(&dir_path, false)?;
            search_in_directory(&dir_path, Arc::new(regex))?;
        }
        Commands::Extract { file_path, pattern } => {
            let regex = Regex::new(&pattern)
                .with_context(|| format!("Invalid regex pattern {}", pattern))?;
            validate_input(&file_path, true)?;
            let lines = read_file(&file_path)?;
            extract_text(&lines, &regex);
        }
        Commands::Replace {
            file_path,
            pattern,
            replacement,
        } => {
            let regex = Regex::new(&pattern)
                .with_context(|| format!("Invalid regex pattern {}", pattern))?;
            validate_input(&file_path, true)?;
            let lines = read_file(&file_path)?;
            replace_text(&lines, &file_path, &regex, &replacement)?;
        }
        Commands::Stats { file_path } => {
            validate_input(&file_path, true)?;
            let lines = read_file(&file_path)?;
            print_text_stats(&lines);
        }
        Commands::Format {
            file_path,
            format_option,
        } => {
            validate_input(&file_path, true)?;
            let lines = read_file(&file_path)?;
            format_text(&lines, &format_option);
        }
        Commands::Validate {
            file_path,
            validation_pattern,
        } => {
            let regex = Regex::new(&validation_pattern)
                .with_context(|| format!("Invalid regex pattern {}", validation_pattern))?;
            validate_input(&file_path, true)?;
            let lines = read_file(&file_path)?;
            validate_text(&lines, &regex);
        }
    }

    Ok(())
}
