use anyhow::Result;
use async_zip::tokio::write::ZipFileWriter;
use async_zip::{Compression, ZipEntryBuilder};
use futures_lite::io::AsyncWriteExt;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub fn walk_dir(root_path_str: &str, relative_path_str: &str) -> Result<Vec<String>> {
    // find all files in ../../src/handlers
    let mut files: Vec<String> = vec![];
    let dir = Path::new(root_path_str).join(relative_path_str);
    let dir_content = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();

    for entry_path in dir_content {
        let metadata = std::fs::metadata(&entry_path).unwrap();
        if metadata.is_dir() {
            walk_dir(root_path_str, entry_path.to_str().unwrap())?
                .iter()
                .for_each(|f| files.push(f.to_string()));
        } else {
            files.push(String::from(
                entry_path
                    .strip_prefix(root_path_str)
                    .unwrap()
                    .to_str()
                    .unwrap(),
            ));
        }
    }
    Ok(files)
}

const BUFFER_SIZE: usize = 100_000_000;
pub async fn write_file_to_zip(
    writer: &mut ZipFileWriter<File>,
    src_file_path: &PathBuf,
    to_path: &str,
) -> Result<u64> {
    let file_size = std::fs::metadata(&src_file_path)?.len();
    log::info!(
        "Writing File {:?} (Size: {:.2} MB) To Zip",
        to_path,
        file_size as f64 / 1_048_576.0
    );

    let time: Instant = std::time::Instant::now();

    let builder = ZipEntryBuilder::new(to_path.into(), Compression::Deflate);

    // if less than 100 MB read all
    if file_size < BUFFER_SIZE as u64 {
        let data = tokio::fs::read(src_file_path).await?;
        writer.write_entry_whole(builder, &data).await?;
    } else {
        let mut entry_writer = writer.write_entry_stream(builder).await?;

        let mut file = tokio::fs::File::open(src_file_path).await?;
        let mut buffer = Vec::with_capacity(BUFFER_SIZE);

        file.read_buf(&mut buffer).await?;
        while !buffer.is_empty() {
            entry_writer.write_all(&buffer).await?;
            buffer.clear();
            file.read_buf(&mut buffer).await?;
        }
        entry_writer.close().await.unwrap();
    }

    log::info!(
        "File {:?} (Size: {:.2} MB) Zip Cost {:.3} s Average Speed: {:.2} MB/s",
        to_path,
        file_size as f64 / 1_048_576.0,
        time.elapsed().as_secs_f32(),
        file_size as f64 / time.elapsed().as_secs_f64() / 1_048_576.0
    );
    Ok(file_size)
}

pub async fn zip_folder(input_path: &Path, out_path: &Path) -> anyhow::Result<()> {
    let out_file = File::create(out_path).await?;
    let time: Instant = std::time::Instant::now();
    let mut writer: ZipFileWriter<File> = ZipFileWriter::with_tokio(out_file);

    let mut total_size = 0_u64;
    let files = walk_dir(input_path.to_str().unwrap(), "").unwrap();
    for file in files {
        let file_path = input_path.join(file);
        let file_relative_path = file_path.strip_prefix(input_path).unwrap();
        total_size += write_file_to_zip(
            &mut writer,
            &file_path,
            file_relative_path.as_os_str().to_str().unwrap(),
        )
        .await?;
    }
    writer.close().await?;
    log::info!(
        "Zip Created In {:?} Average Speed: {:.2} MB/s",
        time.elapsed(),
        total_size as f64 / time.elapsed().as_secs_f64() / 1_048_576.0
    );
    std::result::Result::Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    log::info!("Starting zip file");

    let exe_path = std::env::current_exe().unwrap();
    let exe_folder = exe_path.parent().unwrap();

    let input_path = exe_folder.join("foo");
    let out_path = exe_folder.join("foo.zip");

    let result = zip_folder(&input_path, &out_path).await;
    if result.is_err() {
        log::error!("Error creating zip file: {:?}", result.err());
    }
}
