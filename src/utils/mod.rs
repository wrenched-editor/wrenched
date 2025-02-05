use std::path::Path;

pub fn load_font_blobs_dir<P>(path: P) -> std::io::Result<Vec<Vec<u8>>>
where
    P: AsRef<Path>,
{
    let paths = std::fs::read_dir(path)?;
    let mut blobs = Vec::new();
    for entry in paths {
        let entry = entry?;
        if !entry.metadata()?.is_file() {
            continue;
        }
        let path = entry.path();
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| !["ttf", "otf", "ttc", "otc"].contains(&ext))
            .unwrap_or(true)
        {
            continue;
        }
        let font_data = std::fs::read(&path)?;
        blobs.push(font_data);
    }
    Ok(blobs)
}
