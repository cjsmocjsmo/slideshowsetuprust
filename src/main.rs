use clap::Parser;
use rusqlite::{params, Connection};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(about = "Scan a directory of JPEG images and populate a SQLite database")]
struct Args {
    /// Path to the SQLite database file
    #[arg(long)]
    db_path: String,

    /// Directory to scan for JPEG images
    #[arg(long)]
    image_dir: String,

    /// Base directory prefix to strip when constructing HTTP paths
    #[arg(long)]
    image_base_dir: String,

    /// HTTP path prefix to prepend after stripping the base directory
    #[arg(long)]
    http_prefix: String,

    /// Whether to delete all existing rows before inserting (true or false)
    #[arg(long)]
    reset_db: bool,
}

/// Determine the orientation of an image based on its dimensions.
/// Returns (width, height, orientation_string).
fn img_orient<P: AsRef<Path>>(img_path: P) -> std::result::Result<(u32, u32, &'static str), String> {
    match image::image_dimensions(&img_path) {
        Ok((width, height)) => {
            let orientation = if width > height {
                "landscape"
            } else if width < height {
                "portrait"
            } else {
                "square"
            };

            Ok((width, height, orientation))
        }
        Err(e) => Err(format!(
            "Error processing image {}: {}",
            img_path.as_ref().display(),
            e
        )),
    }
}

/// Create the images table in the SQLite database if it doesn't exist.
fn create_img_db_table(db_path: &Path) -> rusqlite::Result<()> {
    let conn = Connection::open(db_path)?;

    let create_table_sql = "
    CREATE TABLE IF NOT EXISTS images (
        Name TEXT,
        Path TEXT,
        Http TEXT,
        Idx INTEGER,
        Orientation TEXT,
        Width INTEGER,
        Height INTEGER
    );";

    conn.execute(create_table_sql, [])?;
    Ok(())
}

/// Convert file system path to HTTP path by replacing the base directory.
fn create_http_path(fpath: &str, image_base_dir: &str, http_prefix: &str) -> String {
    let normalized_base = image_base_dir.trim_end_matches('/');
    let normalized_prefix = if http_prefix.ends_with('/') {
        http_prefix.to_string()
    } else {
        format!("{}/", http_prefix)
    };

    if let Some(suffix) = fpath.strip_prefix(normalized_base) {
        let cleaned_suffix = suffix.trim_start_matches('/');
        let mut out = String::with_capacity(cleaned_suffix.len() + normalized_prefix.len());
        out.push_str(&normalized_prefix);
        out.push_str(cleaned_suffix);
        out
    } else {
        fpath.to_owned()
    }
}

/// Walk through the directory, find JPEG images, and insert their data into the database.
fn walk_img_dir(
    db_path: &Path,
    directory: &Path,
    image_base_dir: &str,
    http_prefix: &str,
    reset_db: bool,
) -> Result<(), rusqlite::Error> {
    let mut idx: i32 = 0;
    let mut failed_count: usize = 0;
    let mut failed_samples: Vec<String> = Vec::new();

    let mut conn = Connection::open(db_path)?;
    conn.execute_batch(
        "
        PRAGMA synchronous = OFF;
        PRAGMA journal_mode = MEMORY;
        PRAGMA temp_store = MEMORY;
        PRAGMA cache_size = -65536;
        ",
    )?;

    let tx = conn.transaction()?; // Using a transaction for significantly faster batch inserts
    if reset_db {
        tx.execute("DELETE FROM images", [])?;
    }

    let mut stmt = tx.prepare(
        "
        INSERT INTO images (Name, Path, Http, Idx, Orientation, Width, Height)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ",
    )?;

    for entry in WalkDir::new(directory).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("jpg") {
                    idx += 1;
                    let file_path_str = path.to_string_lossy().into_owned();
                    let file_name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();

                    match img_orient(path) {
                        Ok((width, height, orientation)) => {
                            let http_path = create_http_path(&file_path_str, image_base_dir, http_prefix);

                            if stmt
                                .execute(params![
                                    file_name,
                                    file_path_str,
                                    http_path,
                                    idx,
                                    orientation,
                                    width,
                                    height,
                                ])
                                .is_err()
                            {
                                failed_count += 1;
                                if failed_samples.len() < 10 {
                                    failed_samples.push(path.to_string_lossy().into_owned());
                                }
                            }
                        }
                        Err(_) => {
                            failed_count += 1;
                            if failed_samples.len() < 10 {
                                failed_samples.push(file_path_str);
                            }
                        }
                    }
                }
            }
        }
    }

    drop(stmt);

    tx.commit()?;

    // Print summary of failed images
    println!("\n--- Summary ---");
    if failed_count > 0 {
        println!("Failed to process {} image(s)", failed_count);
        if !failed_samples.is_empty() {
            println!("Sample failures (first {}):", failed_samples.len());
        }
        for failed_img in failed_samples {
            println!("  - {}", failed_img);
        }
    } else {
        println!("All images processed successfully!");
    }

    Ok(())
}

fn main() {
    let cfg = Args::parse();

    println!(
        "Setup config: db_path={}, image_dir={}, image_base_dir={}, http_prefix={}, reset_db={}",
        cfg.db_path, cfg.image_dir, cfg.image_base_dir, cfg.http_prefix, cfg.reset_db
    );

    let db_path = Path::new(&cfg.db_path);
    let image_dir = Path::new(&cfg.image_dir);

    // Ensure parent directory for database exists
    if let Some(parent) = Path::new(db_path).parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("Failed to create database directory: {}", e);
            return;
        }
    }

    if let Err(e) = create_img_db_table(db_path) {
        eprintln!("Failed to create table: {}", e);
        return;
    }

    if let Err(e) = walk_img_dir(
        db_path,
        image_dir,
        &cfg.image_base_dir,
        &cfg.http_prefix,
        cfg.reset_db,
    ) {
        eprintln!("Database error: {}", e);
        return;
    }

    println!("Database setup complete.");
}
