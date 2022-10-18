use std::convert::Into;
use std::rc::Rc;
use glutin::{ContextWrapper, GlProfile, PossiblyCurrent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::{Window, WindowBuilder};
use gl::types::*;
use glutin::dpi::{LogicalPosition, Position};
use glutin::event::{ElementState, Event, Ime, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent};
use skia_safe::gpu::{BackendRenderTarget, DirectContext, SurfaceOrigin};
use skia_safe::gpu::gl::{Format, FramebufferInfo};
use skia_safe::{Canvas, Color, Color4f, ColorType, Data, Font, FontMgr, FontStyle, Matrix, Paint, PaintStyle, Picture, PictureRecorder, Point, Rect, Size, Surface, TextBlob, TextBlobBuilder, Vector};
use skia_safe::PaintCap::Butt;
use crate::caribou::basic::{Button, ButtonData, ButtonState, Layout};
use crate::caribou::draw;
use crate::caribou::draw::{Batch, BatchOp, Brush, FontSlant, Material, Path, PathOp, TextAlignment, Transform};
use crate::caribou::math::IntPair;
use crate::caribou::skia::draw::{skia_read_pict, skia_render_batch};

type WindowedContext = ContextWrapper<PossiblyCurrent, Window>;

pub fn skia_build_picture<F>(op: F) -> Picture where F: Fn(&mut Canvas) {
    let mut rec = PictureRecorder::new();
    {
        let canvas = rec.begin_recording(
            Rect::default(), None);
        op(canvas);
    }
    rec.finish_recording_as_picture(
        Some(&Rect::new(0.0, 0.0, 1.0, 1.0))).unwrap()
}

pub struct SkiaEnv {
    pub(crate) surface: Surface,
    pub(crate) gr_context: DirectContext,
    pub(crate) windowed_context: WindowedContext,
}

pub(crate) static mut SKIA_ENV: Option<SkiaEnv> = None;

static mut MOUSE_POS: IntPair = IntPair::new(0, 0);

fn set_skia_env(env: SkiaEnv) {
    unsafe {
        SKIA_ENV = Some(env);
    }
}

fn get_skia_env() -> &'static mut SkiaEnv {
    unsafe {
        SKIA_ENV.as_mut().unwrap()
    }
}

pub fn skia_bootstrap() {
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

    let mut surface = create_surface(&windowed_context, &fb_info, &mut gr_context);
    let sf = windowed_context.window().scale_factor() as f32;
    //println!("{}", sf);

    windowed_context.window().set_ime_allowed(true);
    windowed_context.window().set_ime_position(Position::Logical((100.0, 100.0).into()));

    let mut frame = 0;

    // Guarantee the drop order inside the FnMut closure. `WindowedContext` _must_ be dropped after
    // `DirectContext`.
    //
    // https://github.com/rust-skia/rust-skia/issues/476

    let mut batch = Batch::new();
    batch.add(BatchOp::Pict {
        transform: Transform::default(),
        pict: skia_read_pict("C:\\Users\\raida\\Pictures\\kirby.jpg"),
    });
    batch.add(BatchOp::Path {
        transform: Transform::default(),
        path: Path::from_vec(vec![
            PathOp::Line((0.0, 1080.0 / 2.0).into(),
                         (1920.0, 1080.0 / 2.0).into()),
            PathOp::Line((1920.0 / 2.0, 0.0).into(),
                         (1920.0 / 2.0, 1080.0).into()),
        ]),
        brush: Brush {
            stroke_mat: Material::Solid(0.0, 0.0, 0.0, 1.0),
            fill_mat: Material::Transparent,
            stroke_width: 1.0
        }
    });
    batch.add(BatchOp::Text {
        transform: Transform {
            translate: (1920.0 / 2.0, 1080.0 / 2.0).into(),
            ..Transform::default()
        },
        text: Rc::new("Hello, World".to_string()),
        font: draw::Font {
            family: Rc::new("Arial".to_string()),
            size: 72.0,
            weight: 400,
            slant: FontSlant::Normal
        },
        alignment: TextAlignment::Center,
        brush: Brush {
            stroke_mat: Material::Solid(0.0, 0.0, 0.0, 1.0),
            fill_mat: Material::Solid(0.0, 0.0, 0.0, 1.0),
            stroke_width: 1.0
        }
    });

    let button1 = Button::create();
    Button::interpret(&button1).unwrap().apply_default_style();
    let button2 = Button::create();
    button2.position.set((50.0, 20.0).into());
    Button::interpret(&button2).unwrap().apply_default_style();
    let layout = Layout::create();
    Layout::interpret(&layout).unwrap().children.borrow_mut().push(button1);
    Layout::interpret(&layout).unwrap().children.borrow_mut().push(button2);
    layout.size.set((640.0, 400.0).into());


    set_skia_env(SkiaEnv {
        surface,
        gr_context,
        windowed_context,
    });

    el.run(move |event, _, control_flow| {
        let env = get_skia_env();
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
                WindowEvent::CursorEntered { .. } => {
                    println!("Cursor entered");
                    layout.on_mouse_enter.broadcast();
                }
                WindowEvent::CursorLeft { .. } => {
                    println!("Cursor left");
                    layout.on_mouse_leave.broadcast();
                }
                WindowEvent::CursorMoved {
                    position,
                    modifiers,
                    ..
                } => {
                    layout.on_mouse_move.broadcast(
                        (position.x as i32, position.y as i32).into());
                }
                WindowEvent::MouseInput {
                    state,
                    button,
                    modifiers,
                    ..
                } => {
                    match button {
                        MouseButton::Left => {
                            match state {
                                ElementState::Pressed => {
                                    layout.on_mouse_down.broadcast();
                                }
                                ElementState::Released => {
                                    layout.on_mouse_up.broadcast();
                                }
                            }
                        }
                        MouseButton::Right => {}
                        MouseButton::Middle => {}
                        MouseButton::Other(_) => {}
                    }
                }
                WindowEvent::Ime(ev) => match ev {
                    Ime::Enabled => {
                        println!("Ime enabled");
                    }
                    Ime::Preedit(pre, pos) => {
                        env.windowed_context.window()
                            .set_ime_position(Position::Logical((100.0, 100.0).into()));
                        println!("Ime preedit: {:?} {:?}", pre, pos);
                    }
                    Ime::Commit(str) => {
                        println!("Ime commit: {:?}", str);
                    }
                    Ime::Disabled => {}
                }
                _ => (),
            },
            Event::RedrawRequested(_) => {
                {
                    let canvas = env.surface.canvas();
                    canvas.clear(Color::WHITE);
                    canvas.save();
                    //canvas.scale((0.5, 0.5));
                    //skia_render_batch(canvas, batch.clone());
                    skia_render_batch(canvas, layout.on_draw.broadcast()[0].clone());
                    // canvas.draw_image(&img, (0.0, 0.0), None);
                    canvas.restore();
                    // let fm = FontMgr::default();
                    // let fs = FontStyle::new(Weight::BOLD, Width::NORMAL, Slant::Upright);
                    // let tf = fm
                    //     .match_family_style("Arial", fs)
                    //     .unwrap();
                    // let mut sf = Font::from_typeface(tf, 24.0);
                    // sf.set_edging(Edging::SubpixelAntiAlias);
                    // let tb = TextBlob::from_str("Hello, world!", &sf).unwrap();
                    // let mut pa = Paint::new(Color4f::new(0.0, 0.0, 0.0, 1.0), None);
                    // pa.set_stroke_width(1.5);
                    // pa.set_style(PaintStyle::Stroke);
                    // let pa2 = Paint::new(Color4f::new(1.0, 0.0, 0.0, 1.0), None);
                    // let (sc, sf) = sf.measure_str("Hello, world!", None);
                    // canvas.save();
                    // canvas.translate((100.0, 100.0));
                    // canvas.rotate(30.0, Some(Point::new(0.0, 0.0)));
                    // canvas.clip_rect(Rect::from_xywh(0.0, 0.0, 100.0, 100.0), Some(ClipOp::Intersect), Some(true));
                    // canvas.draw_rect(Rect::from_xywh(0.0, 0.0, 100.0, 100.0), &pa2);
                    // canvas.restore();
                    // canvas.draw_rect(sf.with_offset((0.0, 20.0)), &pa2);
                    // canvas.draw_text_blob(&tb, (0.0, 20.0), &pa);
                }
                env.surface.canvas().flush();
                env.windowed_context.swap_buffers().unwrap();
            }
            _ => (),
        }
    });
}