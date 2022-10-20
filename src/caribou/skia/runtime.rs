use std::convert::Into;
use std::time::{Duration, Instant};
use glutin::{ContextWrapper, GlProfile, PossiblyCurrent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::{Window, WindowBuilder};
use gl::types::*;
use glutin::dpi::{Position};
use glutin::event::{ElementState, Event, Ime, KeyboardInput, ModifiersState, MouseButton, ScanCode, VirtualKeyCode, WindowEvent};
use log::{info, warn};
use skia_safe::gpu::{BackendRenderTarget, DirectContext, SurfaceOrigin};
use skia_safe::gpu::gl::{Format, FramebufferInfo};
use skia_safe::{Canvas, Color, ColorType, FontMgr, FontStyle, Matrix, Paint, PaintStyle, Picture, PictureRecorder, Point, Rect, Size, Surface, TextBlob, TextBlobBuilder, Vector};
use crate::caribou::basic::{Layout};
use crate::caribou::{Caribou};
use crate::caribou::draw::{BatchConsolidation, BatchOp, Brush, FontSlant, Material, Path, PathOp, TextAlignment, Transform};
use crate::caribou::input::{Key, KeyEvent};
use crate::caribou::math::IntPair;
use crate::caribou::skia::draw::{skia_render_batch};
use crate::caribou::skia::input::gl_virtual_to_key;

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

pub fn skia_gl_set_env(env: SkiaEnv) {
    unsafe {
        SKIA_ENV = Some(env);
    }
}

pub fn skia_gl_get_env() -> &'static mut SkiaEnv {
    unsafe {
        SKIA_ENV.as_mut().unwrap()
    }
}

static mut KEY_RETAIN_VEC: Vec<Key> = Vec::new();

pub fn glut_cb_key_retain_vec() -> &'static mut Vec<Key> {
    unsafe {
        &mut KEY_RETAIN_VEC
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
    skia_gl_set_env(SkiaEnv {
        surface,
        gr_context,
        windowed_context,
    });

    el.run(move |event, _, control_flow| {
        let env = skia_gl_get_env();
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(16));

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
                        scancode,
                        virtual_keycode,
                        modifiers,
                        ..
                    },
                    ..
                } => {
                    println!("Keyboard input: {:?}", virtual_keycode);
                    if modifiers.logo() {
                        if let Some(VirtualKeyCode::Q) = virtual_keycode {
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                    if let Some(vir) = virtual_keycode {
                        let key = gl_virtual_to_key(vir);
                        let ret_vec = glut_cb_key_retain_vec();
                        if ret_vec.contains(&key) {
                            ret_vec.retain(|x| *x != key);
                            Caribou::instance().on_key_up.broadcast(KeyEvent {
                                key,
                                modifiers: vec![]
                            });
                        } else {
                            ret_vec.push(key);
                            Caribou::instance().on_key_down.broadcast(KeyEvent {
                                key,
                                modifiers: vec![]
                            });
                        }
                    }
                    frame += 1;
                    env.windowed_context.window().request_redraw();
                }
                WindowEvent::CursorEntered { .. } => {
                    println!("Cursor entered");
                    Caribou::root_component().on_mouse_enter.broadcast();
                }
                WindowEvent::CursorLeft { .. } => {
                    println!("Cursor left");
                    Caribou::root_component().on_mouse_leave.broadcast();
                }
                WindowEvent::CursorMoved {
                    position,
                    modifiers,
                    ..
                } => {
                    Caribou::root_component().on_mouse_move.broadcast(
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
                                    Caribou::root_component().on_primary_down.broadcast();
                                }
                                ElementState::Released => {
                                    Caribou::root_component().on_primary_up.broadcast();
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
                    canvas.reset_matrix();
                    // canvas.scale((1.25, 1.25)); //TODO: DPI awareness
                    canvas.save();
                    skia_render_batch(canvas, Caribou::root_component().on_draw
                            .broadcast().consolidate());
                    canvas.restore();
                }
                env.surface.canvas().flush();
                env.windowed_context.swap_buffers().unwrap();
            }
            _ => (),
        }
    });
}