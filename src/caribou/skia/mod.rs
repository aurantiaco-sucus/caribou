use glutin::{ContextWrapper, GlProfile, PossiblyCurrent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::{Window, WindowBuilder};
use gl::types::*;
use glutin::event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use skia_safe::gpu::{BackendRenderTarget, DirectContext, SurfaceOrigin};
use skia_safe::gpu::gl::{Format, FramebufferInfo};
use skia_safe::{Canvas, ClipOp, Color, Color4f, ColorType, Font, FontMgr, FontStyle, Matrix, Paint, Picture, PictureRecorder, Point, Rect, Size, Surface, TextBlob, TextBlobBuilder, Vector};
use skia_safe::canvas::SaveLayerRec;
use skia_safe::font::Edging;
use skia_safe::wrapper::PointerWrapper;

type WindowedContext = ContextWrapper<PossiblyCurrent, Window>;

pub fn build_picture<F>(op: F) -> Picture where F: Fn(&mut Canvas) {
    let mut rec = PictureRecorder::new();
    {
        let canvas = rec.begin_recording(
            Rect::default(), None);
        op(canvas);
    }
    rec.finish_recording_as_picture(
        Some(&Rect::new(0.0, 0.0, 1.0, 1.0))).unwrap()
}

pub fn bootstrap() {
    let el = EventLoop::new();
    let wb = WindowBuilder::new().with_title("Caribou");

    let cb = glutin::ContextBuilder::new()
        .with_depth_buffer(0)
        .with_stencil_buffer(8)
        .with_pixel_format(24, 8)
        .with_gl_profile(GlProfile::Core);
    #[cfg(not(feature = "wayland"))]
        let cb = cb
        .with_double_buffer(Some(true));

    let windowed_context = cb.build_windowed(wb, &el).unwrap();

    let windowed_context = unsafe { windowed_context.make_current().unwrap() };
    let pixel_format = windowed_context.get_pixel_format();

    println!(
        "Pixel format of the window's GL context: {:#?}",
        pixel_format
    );

    gl::load_with(|s| windowed_context.get_proc_address(s));

    let mut gr_context = DirectContext::new_gl(None, None).unwrap();

    let fb_info = {
        let mut fboid: GLint = 0;
        unsafe { gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid) };

        FramebufferInfo {
            fboid: fboid.try_into().unwrap(),
            format: Format::RGBA8.into(),
        }
    };

    windowed_context
        .window();

    fn create_surface(
        windowed_context: &WindowedContext,
        fb_info: &FramebufferInfo,
        gr_context: &mut DirectContext,
    ) -> Surface {
        let pixel_format = windowed_context.get_pixel_format();
        let size = windowed_context.window().inner_size();
        let backend_render_target = BackendRenderTarget::new_gl(
            (
                size.width.try_into().unwrap(),
                size.height.try_into().unwrap(),
            ),
            pixel_format.multisampling.map(|s| s.try_into().unwrap()),
            pixel_format.stencil_bits.try_into().unwrap(),
            *fb_info,
        );
        Surface::from_backend_render_target(
            gr_context,
            &backend_render_target,
            SurfaceOrigin::BottomLeft,
            ColorType::RGBA8888,
            None,
            None,
        )
            .unwrap()
    }

    let surface = create_surface(&windowed_context, &fb_info, &mut gr_context);
    // let sf = windowed_context.window().scale_factor() as f32;
    // surface.canvas().scale((sf, sf));

    let mut frame = 0;

    // Guarantee the drop order inside the FnMut closure. `WindowedContext` _must_ be dropped after
    // `DirectContext`.
    //
    // https://github.com/rust-skia/rust-skia/issues/476
    struct Env {
        surface: Surface,
        gr_context: skia_safe::gpu::DirectContext,
        windowed_context: WindowedContext,
    }

    let mut env = Env {
        surface,
        gr_context,
        windowed_context,
    };

    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        #[allow(deprecated)]
        match event {
            Event::LoopDestroyed => {}
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(physical_size) => {
                    env.surface =
                        create_surface(&env.windowed_context, &fb_info, &mut env.gr_context);
                    env.windowed_context.resize(physical_size)
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput {
                    input:
                    KeyboardInput {
                        virtual_keycode,
                        modifiers,
                        ..
                    },
                    ..
                } => {
                    if modifiers.logo() {
                        if let Some(VirtualKeyCode::Q) = virtual_keycode {
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                    frame += 1;
                    env.windowed_context.window().request_redraw();
                }
                _ => (),
            },
            Event::RedrawRequested(_) => {
                {
                    let canvas = env.surface.canvas();
                    canvas.clear(Color::WHITE);
                    let fm = FontMgr::default();
                    let tf = fm
                        .match_family_style("Arial", FontStyle::default())
                        .unwrap();
                    let mut sf = Font::from_typeface(tf, 24.0);
                    sf.set_edging(Edging::SubpixelAntiAlias);
                    let tb = TextBlob::from_str("Hello, world!", &sf).unwrap();
                    let pa = Paint::new(Color4f::new(0.0, 0.0, 0.0, 1.0), None);
                    let pa2 = Paint::new(Color4f::new(1.0, 0.0, 0.0, 1.0), None);
                    let (sc, sf) = sf.measure_str("Hello, world!", None);
                    canvas.save();
                    canvas.translate((100.0, 100.0));
                    canvas.clip_rect(Rect::from_xywh(0.0, 0.0, 100.0, 100.0), Some(ClipOp::Intersect), Some(true));
                    canvas.draw_rect(Rect::from_xywh(0.0, 0.0, 100.0, 100.0), &pa2);
                    canvas.restore();
                    canvas.draw_rect(sf.with_offset((0.0, 20.0)), &pa2);
                    canvas.draw_text_blob(&tb, (0.0, 20.0), &pa);
                }
                env.surface.canvas().flush();
                env.windowed_context.swap_buffers().unwrap();
            }
            _ => (),
        }
    });
}