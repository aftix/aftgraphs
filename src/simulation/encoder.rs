use crossbeam::{channel, select};
use dcv_color_primitives::{convert_image, get_buffers_size, ColorSpace, ImageFormat, PixelFormat};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    thread::{self, JoinHandle},
};
use x264::{Encoder, Param, Picture};

/// Starts a video encoder in the background
/// Returns a sending channel to send frames to
/// The frames shouldn't be changed from the GPU buffer.
/// Close the channel to signal the end.
pub fn encoder(
    size: (u32, u32),
    delta_t: f64,
    out_file: impl AsRef<Path>,
) -> (
    channel::Sender<Vec<u8>>,
    channel::Sender<()>,
    JoinHandle<()>,
) {
    let (send, recv) = channel::bounded(8);
    let (send_finished, recv_finished) = channel::bounded(1);
    let out_file = out_file.as_ref().to_owned();

    let handle = thread::spawn(move || {
        let mut params = Param::new()
            .set_dimension(size.0 as usize, size.1 as usize)
            .param_parse("repeat_headers", "1")
            .and_then(|params| params.param_parse("annexb", "1"))
            .and_then(|params| params.param_parse("fps", &(1.0 / delta_t).to_string()))
            .and_then(|params| params.apply_profile("high"))
            .unwrap();

        let picture = Picture::from_param(&params).unwrap();
        let encoder = Encoder::open(&mut params).unwrap();

        let handler = EncoderHandler {
            size: (size.0 as usize, size.1 as usize),
            out_file,
            channel: recv,
            finished: recv_finished,
            picture,
            encoder,
        };

        handler.encoding_loop();
    });

    (send, send_finished, handle)
}

struct EncoderHandler {
    size: (usize, usize),
    out_file: PathBuf,
    channel: channel::Receiver<Vec<u8>>,
    finished: channel::Receiver<()>,
    picture: Picture,
    encoder: Encoder,
}

impl EncoderHandler {
    fn encoding_loop(mut self) {
        let mut out_file = File::create(self.out_file.clone())
            .expect("aftgraphs::simulation::encoder::EncoderHandler: Failed to create output file");

        let bytes_per_row = std::mem::size_of::<u32>() * self.size.0;
        let missing_bytes = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize
            - (bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize);
        let bytes_per_row = bytes_per_row + missing_bytes;

        let mut frame_idx = 0;
        'outer: loop {
            select! {
                recv(self.channel) -> frame => {
                    let frame = match frame {
                        Ok(f) => f,
                        Err(e) => {
                            log::warn!("aftgraphs::simulation::encoder::EncoderHandler: Error recieving frame: {e:?}");
                            continue;
                        },
                    };

                    let encoded_frame = Self::encode_frame(self.size, bytes_per_row, frame);

                    self.picture = self.picture.set_timestamp(frame_idx as i64);
                    self.picture
                        .as_mut_slice(0)
                        .unwrap()
                        .copy_from_slice(encoded_frame.0.as_slice());
                    self.picture
                        .as_mut_slice(1)
                        .unwrap()
                        .copy_from_slice(encoded_frame.1.as_slice());
                    self.picture
                        .as_mut_slice(2)
                        .unwrap()
                        .copy_from_slice(encoded_frame.2.as_slice());

                    if let Some((nal, _, _)) = self.encoder.encode(&self.picture).unwrap() {
                        out_file.write_all(nal.as_bytes()).expect("aftgraphs::simulation::encoder::EncoderHandler: Failed to write frame to output file");
                    }

                    frame_idx += 1;
                }
                recv(self.finished) -> _ => {
                    break 'outer;
                }
            }
        }

        while self.encoder.delayed_frames() {
            match self.encoder.encode(None) {
                Ok(Some((nal, _, _))) => out_file.write_all(nal.as_bytes()).expect("aftgraphs::simulation::encoder::EncoderHandler: Failed to write frame to output file"),
                Ok(None) => log::info!("aftgraphs::simulation::encoder::EncoderHandler: delayed frame encoding resulted in None"),
                Err(e) => log::warn!("aftgraphs::simulation::encoder::EncoderHandler: delayed frame encoding resulted in Err: {e}"),
            }
        }
    }

    fn encode_frame(
        (width, height): (usize, usize),
        bytes_per_row: usize,
        mut frame: Vec<u8>,
    ) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let u32_size = std::mem::size_of::<u32>();
        let expected_bytes = u32_size * width;

        // Remove the padding bytes WGPU required
        for row in (0..height).rev() {
            let row_start = bytes_per_row * row;
            let row_end = row_start + bytes_per_row;
            let excess_start = row_start + expected_bytes;

            frame.drain(excess_start..row_end);
        }

        // Now frame is a correctly sized srgb buffer
        // Need to convert to yuv420

        let source_format = ImageFormat {
            pixel_format: PixelFormat::Rgb,
            color_space: ColorSpace::Rgb,
            num_planes: 1,
        };
        let bgra_format = ImageFormat {
            pixel_format: PixelFormat::Bgra,
            color_space: ColorSpace::Rgb,
            num_planes: 1,
        };
        let i420_format = ImageFormat {
            pixel_format: PixelFormat::I420,
            color_space: ColorSpace::Bt601,
            num_planes: 3,
        };

        let rgb_frame: Vec<_> = frame
            .chunks_exact(4)
            .flat_map(|chunk| [chunk[0], chunk[1], chunk[2]])
            .collect();
        let mut bgra_frame = vec![0; frame.len()];

        convert_image(
            width as u32,
            height as u32,
            &source_format,
            None,
            &[&rgb_frame],
            &bgra_format,
            None,
            &mut [&mut bgra_frame],
        )
        .unwrap();

        let dst_sizes = &mut [0usize; 3];
        get_buffers_size(width as u32, height as u32, &i420_format, None, dst_sizes).unwrap();

        let mut out_planes = (
            vec![0; dst_sizes[0]],
            vec![0; dst_sizes[1]],
            vec![0; dst_sizes[2]],
        );

        convert_image(
            width as u32,
            height as u32,
            &bgra_format,
            None,
            &[bgra_frame.as_slice()],
            &i420_format,
            None,
            &mut [&mut out_planes.0, &mut out_planes.1, &mut out_planes.2],
        )
        .unwrap();

        out_planes
    }
}
