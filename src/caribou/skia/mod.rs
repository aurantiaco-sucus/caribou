use std::any::Any;
use skia_safe::{Canvas, ClipOp, Codec, Color, Data, FontMgr, FontStyle, Image, Paint, PaintStyle, Rect, TextBlob};
use std::cell::Ref;
use std::fmt::{Debug, Formatter};
use skia_safe::font_style::{Slant, Weight, Width};
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex, RwLock};
use crate::caribou::batch::{Batch, BatchOp, Brush, Font, FontSlant, Material, Path, PathOp, Pict, PictImpl, TextAlignment, Transform};
use crate::caribou::math::ScalarPair;
use crate::caribou::skia::runtime::SKIA_ENV;

pub mod runtime;
pub mod input;

pub fn skia_render_batch(canvas: &mut Canvas, batch: Batch) {
    for op in batch.data().unwrap().iter() {
        match op {
            BatchOp::Pict { transform, pict } => {
                let save = canvas.save();
                skia_apply_transform(canvas, transform);
                let image_guard = pict.data().unwrap();
                let image = image_guard.get();
                let image: &Image = image.downcast_ref().unwrap();
                canvas.draw_image(image, (0.0, 0.0), None);
                canvas.restore_to_count(save);
            }
            BatchOp::Path { transform, path, brush } => {
                let save = canvas.save();
                skia_apply_transform(canvas, transform);
                let (stroke, fill) = skia_make_paint(&brush);
                let path = skia_make_path(path);
                canvas.draw_path(&path, &fill);
                canvas.draw_path(&path, &stroke);
                canvas.restore_to_count(save);
            }
            BatchOp::Text {
                transform,
                text,
                font,
                alignment,
                brush
            } => {
                if text.is_empty() {
                    continue;
                }
                let save = canvas.save();
                skia_apply_transform(canvas, transform);
                let (stroke, fill) = skia_make_paint(&brush);
                let skia_font = skia_make_font(font);
                //let skia_font = skia_default_font();
                let (_, bounds) = skia_font
                    .measure_str(&*text, None);
                canvas.translate(match alignment {
                    TextAlignment::Origin => (0.0, bounds.height()),
                    TextAlignment::Center => (-bounds.width() / 2.0, bounds.height() / 2.0),
                });
                let blob = TextBlob::from_str(&*text, &skia_font).unwrap();
                if let Material::Transparent = brush.stroke_mat {} else {
                    canvas.draw_text_blob(&blob, (0.0, 0.0), &stroke);
                }
                if let Material::Transparent = brush.fill_mat {} else {
                    canvas.draw_text_blob(&blob, (0.0, 0.0), &fill);
                }
                canvas.restore_to_count(save);
            }
            BatchOp::Batch { transform, batch } => {
                let save = canvas.save();
                skia_apply_transform(canvas, transform);
                // println!("{:?}", canvas.local_to_device_as_3x3());
                skia_render_batch(canvas, batch.clone());
                canvas.restore_to_count(save);
            }
        }
    }
}

pub fn skia_apply_transform(canvas: &mut Canvas, transform: &Transform) {
    canvas.translate((transform.translate.x,
                      transform.translate.y));
    if let Some(ScalarPair{ x, y }) = transform.clip_size {
        canvas.clip_rect(Rect::from_wh(x, y),
                         ClipOp::Intersect,
                         true);
    }
    canvas.scale((transform.scale.x, transform.scale.y));
    canvas.rotate(transform.rotate, None);
}

pub fn skia_make_path(path: &Path) -> skia_safe::Path {
    let mut skia_path = skia_safe::Path::new();
    for op in path.data().unwrap().iter() {
        match op {
            PathOp::MoveTo(pair) => {
                skia_path.move_to((pair.x, pair.y));
            }
            PathOp::LineTo(pair) => {
                skia_path.line_to((pair.x, pair.y));
            }
            PathOp::QuadTo(pair1, pair2) => {
                skia_path.quad_to((pair1.x, pair1.y),
                                  (pair2.x, pair2.y));
            }
            PathOp::CubicTo(pair1, pair2, pair3) => {
                skia_path.cubic_to((pair1.x, pair1.y),
                                   (pair2.x, pair2.y),
                                   (pair3.x, pair3.y));
            }
            PathOp::Close => {
                skia_path.close();
            }
            PathOp::Line(begin, end) => {
                skia_path.move_to((begin.x, begin.y));
                skia_path.line_to((end.x, end.y));
                skia_path.close();
            }
            PathOp::Rect(position, size) => {
                skia_path.add_rect(
                    Rect::from_xywh(position.x, position.y,
                                    size.x, size.y),
                    None);
            }
            PathOp::Oval(position, size) => {
                skia_path.add_oval(
                    Rect::from_xywh(position.x, position.y,
                                    size.x, size.y),
                    None);
            }
        }
    }
    skia_path
}

pub fn skia_make_paint(brush: &Brush) -> (Paint, Paint) {
    let mut stroke_paint = Paint::default();
    stroke_paint.set_style(PaintStyle::Stroke);
    stroke_paint.set_anti_alias(true);
    stroke_paint.set_stroke_width(brush.stroke_width);
    let mut fill_paint = Paint::default();
    fill_paint.set_style(PaintStyle::Fill);
    fill_paint.set_anti_alias(true);
    stroke_paint.set_color(match brush.stroke_mat {
        Material::Transparent => Color::TRANSPARENT,
        Material::Solid(r, g, b, a) => Color::from_argb(
            (a * 255.0) as u8, (r * 255.0) as u8,
            (g * 255.0) as u8, (b * 255.0) as u8),
    });
    fill_paint.set_color(match brush.fill_mat {
        Material::Transparent => Color::TRANSPARENT,
        Material::Solid(r, g, b, a) => Color::from_argb(
            (a * 255.0) as u8, (r * 255.0) as u8,
            (g * 255.0) as u8, (b * 255.0) as u8),
    });
    (stroke_paint, fill_paint)
}

#[derive(Debug)]
pub struct SkiaPict {
    image: Image,
}

impl PictImpl for SkiaPict {
    fn get(&self) -> Box<dyn Any> {
        Box::new(self.image.clone())
    }
}

pub fn skia_read_pict(path: &str) -> Pict {
    let mut img = File::open(path).unwrap();
    let mut buf = Vec::new();
    img.read_to_end(&mut buf).unwrap();
    let mut codec = Codec::from_data(Data::new_copy(&buf)).unwrap();
    let img = codec.get_image(None, None).unwrap();
    Pict::new(Box::new(SkiaPict { image: img }))
}

pub fn skia_make_font(font: &Font) -> skia_safe::Font {
    let mgr = FontMgr::default();
    let style = FontStyle::new(
        Weight::from(font.weight),
        Width::NORMAL,
        match font.slant {
            FontSlant::Normal => Slant::Upright,
            FontSlant::Italic => Slant::Italic,
            FontSlant::Oblique => Slant::Oblique
        });
    let face = mgr
        .match_family_style(&*font.family, style)
        .unwrap();
    skia_safe::Font::from_typeface(face, font.size)
}

pub fn skia_default_font() -> skia_safe::Font {
    skia_safe::Font::default()
}

pub fn skia_request_redraw() {
    unsafe {
        SKIA_ENV.as_ref().unwrap_unchecked().windowed_context.window().request_redraw();
    }
}
