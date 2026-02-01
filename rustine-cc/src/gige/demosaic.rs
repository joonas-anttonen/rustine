pub fn demosaic_bayer_rg8(in_buf: &mut [u8], width: usize, height: usize) -> Vec<u8> {
    // Zero out first and last row
    for x in 0..width {
        in_buf[x] = 0;
        in_buf[(height - 1) * width + x] = 0;
    }
    // Zero out first and last column
    for y in 0..height {
        in_buf[y * width] = 0;
        in_buf[y * width + (width - 1)] = 0;
    }

    let mut out = vec![0u8; width * height * 4];

    let pick_first = |candidates: &[(usize, usize); 4]| -> u8 {
        for &(nx, ny) in candidates.iter() {
            let v = in_buf[ny * width + nx];
            if v != 0u8 {
                return v;
            }
        }
        0u8
    };

    for y in 1..(height - 1) {
        for x in 1..(width - 1) {
            let is_row_even = (y % 2) == 0;
            let is_col_even = (x % 2) == 0;

            let idx = y * width + x;

            let center = in_buf[idx];

            let (r, g, b) = if is_row_even && is_col_even {
                // R pixel
                let r = center;
                let g = pick_first(&[(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
                let b = pick_first(&[
                    (x + 1, y + 1),
                    (x - 1, y + 1),
                    (x + 1, y - 1),
                    (x - 1, y - 1),
                ]);
                (r, g, b)
            } else if is_row_even && !is_col_even {
                // G on R row
                let g = center;
                let r = pick_first(&[(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]);
                let b = pick_first(&[(x, y - 1), (x, y + 1), (x - 1, y), (x + 1, y)]);
                (r, g, b)
            } else if !is_row_even && is_col_even {
                // G on B row
                let g = center;
                let r = pick_first(&[(x, y - 1), (x, y + 1), (x - 1, y), (x + 1, y)]);
                let b = pick_first(&[(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]);
                (r, g, b)
            } else {
                // B pixel
                let b = center;
                let g = pick_first(&[(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]);
                let r = pick_first(&[
                    (x - 1, y - 1),
                    (x + 1, y - 1),
                    (x - 1, y + 1),
                    (x + 1, y + 1),
                ]);
                (r, g, b)
            };

            let base = idx * 4;
            out[base + 0] = r;
            out[base + 1] = g;
            out[base + 2] = b;
            out[base + 3] = 0xffu8;
        }
    }

    out
}

pub fn minimal_demosaic_bayer_rg8(in_buf: &mut [u8], width: usize, height: usize) -> Vec<u8> {
    // Zero out first and last row
    for x in 0..width {
        in_buf[x] = 0;
        in_buf[(height - 1) * width + x] = 0;
    }
    // Zero out first and last column
    for y in 0..height {
        in_buf[y * width] = 0;
        in_buf[y * width + (width - 1)] = 0;
    }

    let mut out = vec![0u8; width * height * 4];

    for y in 1..(height - 1) {
        for x in 1..(width - 1) {
            let is_row_even = (y % 2) == 0;
            let is_col_even = (x % 2) == 0;

            let idx = y * width + x;

            let center = in_buf[idx];

            let (r, g, b) = if is_row_even && is_col_even {
                // R pixel
                let r = center;
                let g = in_buf[y * width + (x + 1)];
                let b = in_buf[(y + 1) * width + (x + 1)];
                (r, g, b)
            } else if is_row_even && !is_col_even {
                // G on R row
                let g = center;
                let r = in_buf[y * width + (x - 1)];
                let b = in_buf[(y - 1) * width + x];
                (r, g, b)
            } else if !is_row_even && is_col_even {
                // G on B row
                let g = center;
                let r = in_buf[(y - 1) * width + x];
                let b = in_buf[y * width + (x - 1)];
                (r, g, b)
            } else {
                // B pixel
                let b = center;
                let g = in_buf[y * width + (x - 1)];
                let r = in_buf[(y - 1) * width + (x - 1)];
                (r, g, b)
            };

            let base = idx * 4;
            out[base + 0] = r;
            out[base + 1] = g;
            out[base + 2] = b;
            out[base + 3] = 0xffu8;
        }
    }

    out
}

/// Demosaic using FFmpeg's swscale (requires rustine::io::ffmpeg feature)
pub fn ffmpeg_demosaic_bayer_rg8(in_buf: &[u8], width: usize, height: usize) -> Vec<u8> {
    rustine::io::ffmpeg::demosaic_bayer_rg8(in_buf, width as u32, height as u32)
        .expect("FFmpeg demosaic failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_demosaic_implementations() {
        // Create a simple test pattern: alternating RGGB Bayer pattern
        let width = 256;
        let height = 256;
        let mut bayer_data = vec![0u8; width * height];

        // Fill with a gradient pattern
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                bayer_data[idx] = ((x + y) % 256) as u8;
            }
        }

        // Test our implementation
        let mut bayer_copy1 = bayer_data.clone();
        let start = std::time::Instant::now();
        let result_custom = demosaic_bayer_rg8(&mut bayer_copy1, width, height);
        let duration_custom = start.elapsed();
        println!("Custom demosaic: {:?}", duration_custom);

        // Test minimal implementation
        let mut bayer_copy2 = bayer_data.clone();
        let start = std::time::Instant::now();
        let result_minimal = minimal_demosaic_bayer_rg8(&mut bayer_copy2, width, height);
        let duration_minimal = start.elapsed();
        println!("Minimal demosaic: {:?}", duration_minimal);

        // Test FFmpeg implementation
        let start = std::time::Instant::now();
        let result_ffmpeg = ffmpeg_demosaic_bayer_rg8(&bayer_data, width, height);
        let duration_ffmpeg = start.elapsed();
        println!("FFmpeg demosaic: {:?}", duration_ffmpeg);

        // Verify output sizes are correct
        assert_eq!(result_custom.len(), width * height * 4);
        assert_eq!(result_minimal.len(), width * height * 4);
        assert_eq!(result_ffmpeg.len(), width * height * 4);

        println!("\nPerformance comparison (256x256):");
        println!("  Custom:  {:?}", duration_custom);
        println!("  Minimal: {:?}", duration_minimal);
        println!("  FFmpeg:  {:?}", duration_ffmpeg);
    }
}

    #[test]
    fn compare_demosaic_1920x1080() {
        // Test with HD resolution
        let width = 1920;
        let height = 1080;
        let mut bayer_data = vec![0u8; width * height];

        // Fill with a gradient pattern
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                bayer_data[idx] = ((x + y) % 256) as u8;
            }
        }

        // Test our implementation
        let mut bayer_copy1 = bayer_data.clone();
        let start = std::time::Instant::now();
        let result_custom = demosaic_bayer_rg8(&mut bayer_copy1, width, height);
        let duration_custom = start.elapsed();

        // Test minimal implementation
        let mut bayer_copy2 = bayer_data.clone();
        let start = std::time::Instant::now();
        let result_minimal = minimal_demosaic_bayer_rg8(&mut bayer_copy2, width, height);
        let duration_minimal = start.elapsed();

        // Test FFmpeg implementation
        let start = std::time::Instant::now();
        let result_ffmpeg = ffmpeg_demosaic_bayer_rg8(&bayer_data, width, height);
        let duration_ffmpeg = start.elapsed();

        // Verify output sizes are correct
        assert_eq!(result_custom.len(), width * height * 4);
        assert_eq!(result_minimal.len(), width * height * 4);
        assert_eq!(result_ffmpeg.len(), width * height * 4);

        println!("\nPerformance comparison (1920x1080):");
        println!("  Custom:  {:?}", duration_custom);
        println!("  Minimal: {:?}", duration_minimal);
        println!("  FFmpeg:  {:?}", duration_ffmpeg);
        println!("  FFmpeg speedup vs Minimal: {:.2}x", 
            duration_minimal.as_secs_f64() / duration_ffmpeg.as_secs_f64());
        println!("  FFmpeg speedup vs Custom:  {:.2}x", 
            duration_custom.as_secs_f64() / duration_ffmpeg.as_secs_f64());
    }
