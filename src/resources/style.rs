use indicatif::ProgressStyle;

#[derive(Debug, Clone)]
pub enum ProgressStyleType {
    Bytes,
    Export,
    Import,
}

pub async fn get_pb_style(style: ProgressStyleType) -> ProgressStyle {
    match style {
        ProgressStyleType::Bytes => ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.green/white}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#/-"),
        ProgressStyleType::Export => ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:20.yellow/white}] {percent}% ({eta}) {wide_msg}")
            .unwrap()
            .progress_chars("#/-"),
        ProgressStyleType::Import => ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:20.blue/white}] {percent}% ({eta}) {wide_msg}")
            .unwrap()
            .progress_chars("#/-"),
    }
}
