use std::fs;
use std::path::Path;
use vercel_blob::{
    self,
    client::{
        DownloadCommandOptions, ListBlobResultBlob, ListCommandOptions, PutCommandOptions,
        VercelBlobApi,
    },
};

/// List all blobs for the current commit by using a prefix of `<commit_hash>/`.
pub async fn list_commit_blobs() -> Result<Vec<ListBlobResultBlob>, Box<dyn std::error::Error>> {
    let commit_hash_bytes = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .output()?;
    let commit_hash = String::from_utf8(commit_hash_bytes.stdout)?;
    let commit_hash = commit_hash.trim();

    let client = vercel_blob::client::VercelBlobClient::new();
    let command_options = ListCommandOptions {
        limit: None,
        prefix: Some(format!("{}/", commit_hash)),
        cursor: None,
    };

    let list_of_blobs = client.list(command_options).await?;
    Ok(list_of_blobs.blobs)
}

/// Download all provided blobs into the local `artifacts/` directory.
pub async fn download_blobs_to_artifacts(
    blobs: &[ListBlobResultBlob],
) -> Result<(), Box<dyn std::error::Error>> {
    if blobs.is_empty() {
        return Ok(());
    }

    fs::create_dir_all("artifacts")?;

    let client = vercel_blob::client::VercelBlobClient::new();
    for blob in blobs {
        let download_options = DownloadCommandOptions { byte_range: None };
        let bytes = client.download(&blob.url, download_options).await?;
        let filename = Path::new(&blob.pathname)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("artifact.bin");
        let out_path = Path::new("artifacts").join(filename);
        fs::write(out_path, bytes)?;
    }

    Ok(())
}

pub async fn download_blob(blob_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = vercel_blob::client::VercelBlobClient::new();

    let download_options = DownloadCommandOptions { byte_range: None };

    client.download(blob_url, download_options).await?;

    Ok(())
}

pub fn copy_files(source_dir: &str, dest_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let source_path = Path::new(source_dir);
    let dest_path = Path::new(dest_dir);

    // Create destination directory if it doesn't exist
    fs::create_dir_all(dest_path)?;

    // Read the source directory
    let entries = fs::read_dir(source_path)?;

    for entry in entries {
        let entry = entry?;
        let file_type = entry.file_type()?;

        if file_type.is_file() {
            let source_file = entry.path();
            let file_name = source_file.file_name().unwrap();
            let dest_file = dest_path.join(file_name);

            // Copy the file
            fs::copy(&source_file, &dest_file)?;
            println!("Copied: {:?} -> {:?}", source_file, dest_file);
        }
    }

    Ok(())
}

pub async fn upload_blob(
    blob_path: &str,
    bytes: Vec<u8>,
    content_type: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = vercel_blob::client::VercelBlobClient::new();
    let put_options = PutCommandOptions {
        add_random_suffix: false,
        cache_control_max_age: None,
        content_type: content_type.map(|s| s.to_string()),
    };
    client.put(blob_path, bytes, put_options).await?;
    Ok(())
}

/// Upload all `.wasm` files from `folder_path` to Vercel Blob under
/// `<commit_hash>/<filename.wasm>`.
pub async fn upload_wasm_folder(folder_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Determine current commit hash
    let commit_hash_bytes = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .output()?;
    let commit_hash = String::from_utf8(commit_hash_bytes.stdout)?;
    let commit_hash = commit_hash.trim();

    let dir_iter = fs::read_dir(folder_path)?;
    for entry in dir_iter {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_file() {
            continue;
        }

        let path = entry.path();
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("wasm"))
            .unwrap_or(false)
        {
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };

            let blob_path = format!("{}/{}", commit_hash, file_name);
            let bytes = fs::read(&path)?;
            // Upload with deterministic path and proper content type for WASM
            upload_blob(&blob_path, bytes, Some("application/wasm")).await?;
            log::info!("Uploaded {} to {}", path.display(), blob_path);
        } else if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.eq_ignore_ascii_case("version_map.json") {
                let blob_path = format!("{}/{}", commit_hash, file_name);
                let bytes = fs::read(&path)?;
                upload_blob(&blob_path, bytes, Some("application/json")).await?;
                log::info!("Uploaded {} to {}", path.display(), blob_path);
            }
        }
    }

    Ok(())
}
