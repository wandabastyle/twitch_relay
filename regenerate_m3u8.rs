// Quick script to regenerate the m3u8 file
use std::fs;
use std::path::Path;

mod hls_generator;

fn main() {
    let mp4_path = Path::new(
        "recordings/completed/elara/Season 2026/elara_S2026E0503_vtuber_arrested_for_art_filian_deletes_everything_mori_calliope_.mp4",
    );

    if !mp4_path.exists() {
        println!("File not found: {:?}", mp4_path);
        return;
    }

    let channel_login = "elara";
    let filename = "elara_S2026E0503_vtuber_arrested_for_art_filian_deletes_everything_mori_calliope_.mp4";

    match hls_generator::generate_hls_playlist(mp4_path, channel_login, filename) {
        Ok(playlist) => {
            let m3u8_path = mp4_path.with_extension("m3u8");
            fs::write(&m3u8_path, playlist).expect("Failed to write m3u8");
            println!("Regenerated: {:?}", m3u8_path);
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
