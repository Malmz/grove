
use anyhow::{Result, Context, bail};
use tokio::task;
use tokio::io::AsyncWriteExt;


pub async fn run() -> Result<()> {
    let (_speaker, mut mic) = crate::audio::init()?;
    let encoder = encoder_from_format(mic.format())?;
    let mut frame = vec![0f32; 960 * mic.format().channels as usize];
    let mut out_buf = vec![0u8; 4000];

    let mut file = tokio::fs::File::create("encoded.opus").await?;

    mic.play();

    'outer: loop {
        for slot in frame.iter_mut() {
            if let Ok(val) = mic.recv().await {
                *slot = val;
            } else {
                break 'outer;
            }
        }

        let n = task::block_in_place(|| {
            encoder.encode_float(&frame, &mut out_buf).context("Failed to encode")
        })?;

        file.write_all(&n.to_ne_bytes()).await?;
        file.write_all(&out_buf[..n]).await?;
    }
    file.flush().await?;
    Ok(())
}

fn encoder_from_format(format: &cpal::Format) -> Result<audiopus::coder::Encoder> {
    use audiopus::SampleRate;
    use audiopus::Channels;
    use audiopus::Application;

    let rate = match format.sample_rate.0 {
        8000 => SampleRate::Hz8000,
        12000 => SampleRate::Hz12000,
        16000 => SampleRate::Hz16000,        
        24000 => SampleRate::Hz24000,
        48000 => SampleRate::Hz48000,
        e => bail!("Unsupported rate: {}", e),
    };
    
    let chan = match format.channels {
        1 => Channels::Mono,
        2 => Channels::Stereo,
        e => bail!("Too many channels: {}", e),
    };

    Ok(audiopus::coder::Encoder::new(
        rate,
        chan,
        Application::Voip,
    ).context("failed to create opus encoder")?)
}