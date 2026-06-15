use image::GenericImageView;
use rusqlite::{params, Connection, Result};
use std::fmt;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug)]
struct ImageData {
    name: String,
    path: String,
    http: String,
    idx: i32,
    orientation: String,
    width: u32,
    height: u32,
}

impl fmt::Display for ImageData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ImageData(name='{}', path='{}', http='{}', idx={}, orientation='{}', width={}, height={})",
            self.name, self.path, self.http, self.idx, self.orientation, self.width, self.height
        )
    }
}

/// Determine the orientation of an image based on its dimensions.
/// Returns (width, height, orientation_string).
fn img_orient<P: AsRef<Path>>(img_path: P) -> Result<(u32, u32, String), String> {
    match image::open(&img_path) {
        Ok(img) => {
            let (width, height) = img.dimensions();

            let orientation = if width > height {
                println!("Landscape");
                "landscape".to_string()
            } else if width < height {
                println!("Portrait");
                "portrait".to_string()
            } else {
                println!("Square");
                "square".to_string()
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
fn create_img_db_table<P: AsRef<Path>>(db_path: P) -> Result<()> {
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
fn create_http_path(fpath: &str) -> String {
    fpath.replace("/home/pimedia/Pictures/MASTERPICS/", "/static/")
}

/// Walk through the directory, find JPEG images, and insert their data into the database.
fn walk_img_dir<P: AsRef<Path>>(db_path: P, directory: P) -> Result<(), rusqlite::Error> {
    let mut idx = 0;
    let mut failed_images = Vec::new();

    let mut conn = Connection::open(db_path)?;
    let tx = conn.transaction()?; // Using a transaction for significantly faster batch inserts

    for entry in WalkDir::new(directory).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_file() {
            idx += 1;

            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "jpg" {
                    let file_path_str = path.to_string_lossy().into_owned();
                    let file_name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();

                    match img_orient(path) {
                        Ok((width, height, orientation)) => {
                            let image_data = ImageData {
                                name: file_name,
                                http: create_http_path(&file_path_str),
                                path: file_path_str,
                                idx,
                                orientation,
                                width,
                                height,
                            };

                            println!("{}", image_data);

                            let insert_sql = "
                                INSERT INTO images (Name, Path, Http, Idx, Orientation, Width, Height) 
                                VALUES (?, ?, ?, ?, ?, ?, ?)";

                            if let Err(e) = tx.execute(
                                insert_sql,
                                params![
                                    image_data.name,
                                    image_data.path,
                                    image_data.http,
                                    image_data.idx,
                                    image_data.orientation,
                                    image_data.width,
                                    image_data.height,
                                ],
                            ) {
                                println!("Database insert error for {}: {}", image_data.path, e);
                                failed_images.push(image_data.path);
                            }
                        }
                        Err(e) => {
                            println!("Skipping image: {}", e);
                            failed_images.push(file_path_str);
                        }
                    }
                }
            }
        }
    }

    tx.commit()?;

    // Print summary of failed images
    println!("\n--- Summary ---");
    if !failed_images.is_empty() {
        println!("Failed to process {} image(s):", failed_images.len());
        for failed_img in failed_images {
            println!("  - {}", failed_img);
        }
    } else {
        println!("All images processed successfully!");
    }

    Ok(())
}

fn main() {
    let db_path = "/home/pimedia/go/imagesDB";
    let image_dir = "/home/pimedia/Pictures/MASTERPICS/";

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

    if let Err(e) = walk_img_dir(db_path, image_dir) {
        eprintln!("Database error: {}", e);
    }
}
