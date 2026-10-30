use image::{
    DynamicImage, Rgba, RgbaImage,
    imageops::{self, FilterType},
};
use std::{error::Error, io::Cursor, path::Path};

fn container(source: &RgbaImage, sizes: &[u32], cursor: bool) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut directory = vec![0, 0, if cursor { 2 } else { 1 }, 0];
    directory.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut payload = Vec::new();
    for &size in sizes {
        let frame = imageops::resize(source, size, size, FilterType::Lanczos3);
        // DIB frames are supported for both ICO and CUR on Windows.
        let stride = (size as usize).div_ceil(32) * 4;
        let mut data = Vec::new();
        data.extend_from_slice(&40u32.to_le_bytes());
        data.extend_from_slice(&(size as i32).to_le_bytes());
        data.extend_from_slice(&((size * 2) as i32).to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&32u16.to_le_bytes());
        data.extend_from_slice(&[0; 24]);
        for y in (0..size).rev() {
            for x in 0..size {
                let [r, g, b, a] = frame.get_pixel(x, y).0;
                data.extend_from_slice(&[b, g, r, a]);
            }
        }
        for y in (0..size).rev() {
            let mut row = vec![0; stride];
            for x in 0..size {
                if frame.get_pixel(x, y)[3] < 16 {
                    row[x as usize / 8] |= 0x80 >> (x % 8);
                }
            }
            data.extend_from_slice(&row);
        }
        directory.extend_from_slice(&[size as u8, size as u8, 0, 0]);
        directory.extend_from_slice(&(if cursor { size as u16 / 2 } else { 1u16 }).to_le_bytes());
        directory.extend_from_slice(&(if cursor { size as u16 / 2 } else { 32u16 }).to_le_bytes());
        directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
        directory.extend_from_slice(&((6 + sizes.len() * 16 + payload.len()) as u32).to_le_bytes());
        payload.extend_from_slice(&data);
    }
    directory.extend(payload);
    Ok(directory)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(args.get(1).map(String::as_str).unwrap_or("."));
    let out = root.join("assets/icons");
    std::fs::create_dir_all(&out)?;
    let mut source = image::open(root.join("assets/source/pinwin-ai.jpg"))?.to_rgba8();
    // The model rendered a grayscale checkerboard. Red/charcoal foreground is chromatic;
    // remove only neutral background pixels, leaving transparent RGB black for resizing.
    for pixel in source.pixels_mut() {
        let [r, g, b, _] = pixel.0;
        let spread = r.max(g).max(b) - r.min(g).min(b);
        if spread < 18 && r.max(g).max(b) > 100 {
            *pixel = Rgba([0, 0, 0, 0]);
        }
    }
    source.save(root.join("assets/source/pin.png"))?;
    std::fs::write(
        out.join("app.ico"),
        container(&source, &[16, 20, 24, 32, 48, 64, 128, 256], false)?,
    )?;
    std::fs::write(
        out.join("tray.ico"),
        container(&source, &[16, 20, 24, 32], false)?,
    )?;
    let badge = imageops::resize(&source, 64, 64, FilterType::Lanczos3);
    badge.save(out.join("pinned.png"))?;
    // Native badge currently uses a color-key DIB; flatten only this compatibility asset.
    let small = imageops::resize(&source, 24, 24, FilterType::Lanczos3);
    let mut bmp = image::RgbImage::from_pixel(24, 24, image::Rgb([255, 0, 255]));
    for (x, y, p) in small.enumerate_pixels() {
        if p[3] >= 128 {
            bmp.put_pixel(x, y, image::Rgb([p[0], p[1], p[2]]));
        }
    }
    bmp.save(out.join("pinned.bmp"))?;
    let mut target = RgbaImage::new(128, 128);
    for y in 0..128i32 {
        for x in 0..128i32 {
            let dx = (x - 64).abs();
            let dy = (y - 64).abs();
            let cross =
                (dx <= 3 && (12..=48).contains(&dy)) || (dy <= 3 && (12..=48).contains(&dx));
            let edge = (dx <= 6 && (9..=51).contains(&dy)) || (dy <= 6 && (9..=51).contains(&dx));
            if edge {
                target.put_pixel(
                    x as u32,
                    y as u32,
                    Rgba(if cross {
                        [239, 68, 68, 255]
                    } else {
                        [255, 255, 255, 255]
                    }),
                );
            }
        }
    }
    target.save(root.join("assets/source/target.png"))?;
    std::fs::write(
        out.join("target.cur"),
        container(&target, &[32, 48, 64], true)?,
    )?;
    // ICO preview supports custom-icon settings independently of the CUR resource.
    std::fs::write(
        out.join("target.ico"),
        container(&target, &[32, 48, 64], false)?,
    )?;
    let mut preview = RgbaImage::from_pixel(512, 160, Rgba([245, 246, 248, 255]));
    for y in 80..160 {
        for x in 0..512 {
            preview.put_pixel(x, y, Rgba([32, 35, 42, 255]));
        }
    }
    for (index, size) in [16, 20, 24, 32].into_iter().enumerate() {
        let icon = imageops::resize(&source, size, size, FilterType::Lanczos3);
        imageops::overlay(&mut preview, &icon, 24 + index as i64 * 112, 24);
        imageops::overlay(&mut preview, &icon, 24 + index as i64 * 112, 104);
    }
    preview.save(root.join("assets/preview.png"))?;
    let badge = image::open(out.join("pinned.png"))?.to_rgba8();
    assert!(badge.pixels().any(|p| p[3] == 0) && badge.pixels().any(|p| p[3] == 255));
    // Exercise the independent decoder, including all authored ICO directories.
    let bytes = std::fs::read(out.join("app.ico"))?;
    let _ = DynamicImage::from_decoder(image::codecs::ico::IcoDecoder::new(Cursor::new(bytes))?)?;
    println!("Generated icons, cursor hotspots, badge and preview.");
    Ok(())
}
