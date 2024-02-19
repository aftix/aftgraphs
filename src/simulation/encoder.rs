use crossbeam::channel;
use dcv_color_primitives::{convert_image, get_buffers_size, ColorSpace, ImageFormat, PixelFormat};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering::SeqCst},
    thread,
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
    wait: &'static AtomicUsize,
) -> channel::Sender<Vec<u8>> {
    let (send, recv) = channel::unbounded();
    let out_file = out_file.as_ref().to_owned();

    thread::spawn(move || {
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
            picture,
            encoder,
        };

        handler.encoding_loop(wait);
    });

    send
}

struct EncoderHandler {
    size: (usize, usize),
    out_file: PathBuf,
    channel: channel::Receiver<Vec<u8>>,
    picture: Picture,
    encoder: Encoder,
}

impl EncoderHandler {
    fn encoding_loop(mut self, wait: &AtomicUsize) {
        let mut out_file = File::create(self.out_file.clone())
            .expect("aftgraphs::simulation::encoder::EncoderHandler: Failed to create output file");

        let bytes_per_row = std::mem::size_of::<u32>() * self.size.0;
        let missing_bytes = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize
            - (bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize);
        let bytes_per_row = bytes_per_row + missing_bytes;
        for (frame_idx, frame) in self.channel.into_iter().enumerate() {
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

            wait.fetch_sub(1, SeqCst);
        }

        while self.encoder.delayed_frames() {
            if let Some((nal, _, _)) = self.encoder.encode(None).unwrap() {
                out_file.write_all(nal.as_bytes()).expect("aftgraphs::simulation::encoder::EncoderHandler: Failed to write frame to output file");
            }
        }
    }

    #[allow(dead_code)]
    fn srgb_to_rgb(pixel: u32) -> u32 {
        let s_r = ((pixel & 0xFF000000) >> 24) as u8;
        let s_g = ((pixel & 0x00FF0000) >> 16) as u8;
        let s_b = ((pixel & 0x0000FF00) >> 8) as u8;

        let u8_to_f64 = |val: u8| val as f64 / 255.0;
        let f64_to_u8 = |val: f64| (val * 255.0) as u8;

        let convert = |sval: u8| {
            let val = u8_to_f64(sval);
            let linear = if val <= 0.04045 {
                val / 12.92
            } else {
                ((val + 0.555) / 1.055).powf(2.4)
            };
            f64_to_u8(linear)
        };

        let l_r = convert(s_r) as u32;
        let l_g = convert(s_g) as u32;
        let l_b = convert(s_b) as u32;

        (l_r << 24) + (l_g << 16) + (l_b << 8)
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
