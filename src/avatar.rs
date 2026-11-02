//! Resize the complete centered square, then mask; never crop a tiny 160px fragment first.
pub fn prepare_avatar(image: &image::DynamicImage) -> image::RgbaImage {
    let source = image.to_rgba8();
    let side = source.width().min(source.height());
    let square = image::imageops::crop_imm(
        &source,
        (source.width() - side) / 2,
        (source.height() - side) / 2,
        side,
        side,
    )
    .to_image();
    let mut rgba =
        image::imageops::resize(&square, 128, 128, image::imageops::FilterType::Triangle);
    for (x, y, pixel) in rgba.enumerate_pixels_mut() {
        let distance = ((x as f32 - 63.5).powi(2) + (y as f32 - 63.5).powi(2)).sqrt();
        let coverage = (64.0 - distance).clamp(0.0, 1.0);
        pixel[3] = (pixel[3] as f32 * coverage) as u8;
    }
    rgba
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn large_avatars_keep_the_full_face_and_correct_center_crop() {
        let mut source = image::RgbaImage::from_pixel(800, 800, image::Rgba([0, 0, 255, 255]));
        for y in 0..800 {
            for x in 100..250 {
                source.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }
        let result = prepare_avatar(&image::DynamicImage::ImageRgba8(source));
        assert_eq!(result.dimensions(), (128, 128));
        assert!(
            result.get_pixel(24, 64)[0] > 200,
            "outer face was cropped away"
        );
        assert_eq!(result.get_pixel(64, 64)[2], 255);
        assert_eq!(result.get_pixel(0, 0)[3], 0);
    }
    #[test]
    fn nonsquare_and_small_avatars_are_centered_without_underflow() {
        for (w, h) in [(800, 400), (400, 800), (16, 16)] {
            let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
                w,
                h,
                image::Rgba([0, 255, 0, 255]),
            ));
            assert_eq!(prepare_avatar(&image).get_pixel(64, 64)[1], 255);
        }
    }
}
