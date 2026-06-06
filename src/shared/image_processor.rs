use image::load_from_memory;
use tracing::instrument;

use crate::shared::errors::{AppError, AppResult};

pub const MAIN_SIZE: u32 = 800;
pub const THUMB_SIZE: u32 = 400;

pub struct ProcessedImages {
    pub main_webp: Vec<u8>,
    pub thumb_webp: Vec<u8>,
}

#[instrument(skip(data), fields(size = data.len()))]
pub fn process_image(data: &[u8]) -> AppResult<ProcessedImages> {
    let img = load_from_memory(data).map_err(|e| {
        tracing::warn!(error = %e, "failed to decode image");
        AppError::BadRequest("Invalid image file".to_string())
    })?;

    let (w, h) = (img.width(), img.height());
    let size = w.min(h);
    let x = (w - size) / 2;
    let y = (h - size) / 2;

    let cropped = img.crop_imm(x, y, size, size);

    let main_rgba =
        cropped.resize_exact(MAIN_SIZE, MAIN_SIZE, image::imageops::FilterType::Lanczos3);
    let mut main_webp = Vec::new();
    main_rgba
        .write_to(
            &mut std::io::Cursor::new(&mut main_webp),
            image::ImageFormat::WebP,
        )
        .map_err(|e| {
            tracing::error!(error = %e, "failed to encode main WebP");
            AppError::Internal
        })?;

    let thumb_rgba = cropped.resize_exact(
        THUMB_SIZE,
        THUMB_SIZE,
        image::imageops::FilterType::Lanczos3,
    );
    let mut thumb_webp = Vec::new();
    thumb_rgba
        .write_to(
            &mut std::io::Cursor::new(&mut thumb_webp),
            image::ImageFormat::WebP,
        )
        .map_err(|e| {
            tracing::error!(error = %e, "failed to encode thumb WebP");
            AppError::Internal
        })?;

    Ok(ProcessedImages {
        main_webp,
        thumb_webp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_image() -> Vec<u8> {
        let mut img = image::RgbImage::new(200, 100);
        for x in 0..200 {
            for y in 0..100 {
                img.put_pixel(x, y, image::Rgb([x as u8, y as u8, 128]));
            }
        }
        let mut bytes = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Jpeg,
        )
        .unwrap();
        bytes
    }

    #[test]
    fn test_process_image_square_outputs() {
        let data = create_test_image();
        let result = process_image(&data).unwrap();
        assert!(!result.main_webp.is_empty());
        assert!(!result.thumb_webp.is_empty());
    }

    #[test]
    fn test_process_image_main_is_800() {
        let data = create_test_image();
        let result = process_image(&data).unwrap();
        let decoded = image::load_from_memory(&result.main_webp).unwrap();
        assert_eq!(decoded.width(), MAIN_SIZE);
        assert_eq!(decoded.height(), MAIN_SIZE);
    }

    #[test]
    fn test_process_image_thumb_is_400() {
        let data = create_test_image();
        let result = process_image(&data).unwrap();
        let decoded = image::load_from_memory(&result.thumb_webp).unwrap();
        assert_eq!(decoded.width(), THUMB_SIZE);
        assert_eq!(decoded.height(), THUMB_SIZE);
    }

    #[test]
    fn test_process_image_invalid_data() {
        let result = process_image(b"not an image");
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }
}
