use std::io::Write;
use std::process::{Command, Stdio};

pub fn make_video<F>(
    width: u32,
    height: u32,
    fps: u32,
    duration_sec: u32,
    out_path: &str,
    mut next_frame: F,
) -> std::io::Result<()>
where
    F: FnMut(u32) -> image::RgbImage,
{
    let args = ffmpeg_args(width, height, fps, Codec::Av1, "30", out_path);

    if std::fs::exists(out_path)? {
        std::fs::remove_file(out_path)?;
    }

    let mut ffmpeg = Command::new("ffmpeg")
        .args(&args)
        .stdin(Stdio::piped())
        .spawn()?;

    let stdin = ffmpeg.stdin.as_mut().unwrap();

    for frame_idx in 0..(duration_sec * fps) {
        let img = next_frame(frame_idx);
        stdin.write_all(&img)?;
    }

    let _ = stdin;
    ffmpeg.wait()?;
    Ok(())
}

enum Codec {
    Av1,
}

#[rustfmt::skip]
fn ffmpeg_args(
    width: u32,
    height: u32,
    fps: u32,
    codec: Codec,
    quality: &str,
    output: &str,
) -> Vec<String> {
	let size = format!("{}x{}", width, height);
	let fps = fps.to_string();
    match codec {
        Codec::Av1 => vec![
            "-f", "rawvideo",
            "-pix_fmt", "rgb24",
            "-s", &size,
            "-r", &fps,
            "-i", "-",
            "-c:v", "libaom-av1",
            "-crf", quality,
            "-b:v", "0",
            "-cpu-used", "4",
            output,
        ],
	}
    .into_iter()
    .map(String::from)
    .collect()
}
