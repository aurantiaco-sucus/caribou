use std::borrow::Borrow;
use std::cell::{Ref, RefCell};
use std::rc::{Rc, Weak};
use crate::caribou::draw::{Batch, BatchConsolidation, BatchOp, Brush, Font, FontSlant, Material, Path, PathOp, TextAlignment, Transform};
use crate::caribou::{ComponentRcVec, Event, EventInit, Property, PropertyInit, Subscriber, ZeroArgEvent};
use crate::caribou::math::{IntPair, Region};
use crate::{Caribou, Component};
use crate::caribou::input::Key;

pub struct Layout;

pub struct LayoutData {
    pub children: RefCell<Vec<Rc<Component>>>,
    pub background: Property<Brush>,
    pub border: Property<Brush>,
    pub enabled: Property<bool>,
    cur_hov: RefCell<Vec<Rc<Component>>>,
    cur_pos: RefCell<IntPair>,
}

impl Layout {
    pub fn create() -> Rc<Component> {
        let comp = Component::create();
        comp.on_draw.subscribe(Box::new(|comp| {
            let mut batch = Batch::new();
            let data = comp.data.get_as::<LayoutData>().unwrap();
            data.children.borrow().iter().for_each(|child| {
                let transform = Transform {
                    translate: *child.position.get(),
                    clip_size: Some(*child.size.get()),
                    ..Transform::default()
                };
                let batches = child.on_draw.broadcast();
                for entry in batches {
                    batch.add(BatchOp::Batch {
                        transform,
                        batch: entry,
                    });
                }
            });
            batch
        }));
        comp.on_mouse_move.subscribe(Box::new(|comp, pos| {
            let data = comp.data.get_as::<LayoutData>().unwrap();
            let children = data.children.borrow();
            let mut cur_hov = data.cur_hov.borrow_mut();
            let mut cur_pos = data.cur_pos.borrow_mut();
            *cur_pos = pos;
            let mut new_hov = Vec::new();
            for child in children.iter() {
                let child_pos = *child.position.get();
                let child_size = *child.size.get();
                if Region::origin_size(child_pos, child_size).contains(pos.to_scalar()) {
                    let child_pos = pos - child_pos.to_int();
                    if !cur_hov.comp_contains(child) {
                        child.on_mouse_enter.broadcast();
                    } else {
                        child.on_mouse_move.broadcast(child_pos);
                    }
                    new_hov.push(child.clone());
                }
            }
            for child in cur_hov.iter() {
                if !new_hov.comp_contains(child) {
                    child.on_mouse_leave.broadcast();
                }
            }
            *cur_hov = new_hov;
        }));
        comp.on_mouse_leave.subscribe(Box::new(|comp| {
            let data = comp.data.get_as::<LayoutData>().unwrap();
            let mut cur_hov = data.cur_hov.borrow_mut();
            for child in cur_hov.iter() {
                child.on_mouse_leave.broadcast();
            }
            cur_hov.clear();
        }));
        comp.on_primary_down.subscribe(Box::new(|comp| {
            let data = comp.data.get_as::<LayoutData>().unwrap();
            let cur_hov = data.cur_hov.borrow_mut();
            for child in cur_hov.iter() {
                child.on_primary_down.broadcast();
            }
        }));
        comp.on_primary_up.subscribe(Box::new(|comp| {
            let data = comp.data.get_as::<LayoutData>().unwrap();
            let cur_hov = data.cur_hov.borrow_mut();
            for child in cur_hov.iter() {
                child.on_primary_up.broadcast();
            }
        }));
        comp.data.set(Some(Box::new(LayoutData {
            children: RefCell::new(Vec::new()),
            background: Property::new(Brush::transparent(),
                                      Rc::downgrade(&comp)),
            border: Property::new(Brush::transparent(),
                                  Rc::downgrade(&comp)),
            enabled: Property::new(false,
                                   Rc::downgrade(&comp)),
            cur_hov: RefCell::new(vec![]),
            cur_pos: RefCell::new(Default::default())
        })));
        comp
    }

    pub fn interpret(comp: &Rc<Component>) -> Option<Ref<LayoutData>> {
        comp.data.get_as::<LayoutData>()
    }
}

pub struct Button;

pub enum ButtonState {
    Normal,
    Hover,
    Pressed,
}

pub struct ButtonData {
    pub text: Property<Rc<String>>,
    pub draw_normal: ZeroArgEvent<Batch>,
    pub draw_hover: ZeroArgEvent<Batch>,
    pub draw_pressed: ZeroArgEvent<Batch>,
    pub draw_disabled: ZeroArgEvent<Batch>,
    pub enabled: Property<bool>,
    state: RefCell<ButtonState>,
    focused: RefCell<bool>,
}

impl Button {
    pub fn create() -> Rc<Component> {
        let comp = Component::create();
        comp.on_draw.subscribe(Box::new(|comp| {
            let data = comp.data.get_as::<ButtonData>().unwrap();
            let state = data.state.borrow();
            let enabled =  *data.enabled.get();
            if enabled {
                match &*state {
                    ButtonState::Normal => data.draw_normal.broadcast(),
                    ButtonState::Hover => data.draw_hover.broadcast(),
                    ButtonState::Pressed => data.draw_pressed.broadcast(),
                }.consolidate()
            } else {
                data.draw_disabled.broadcast().consolidate()
            }
        }));
        comp.on_primary_down.subscribe(Box::new(|comp| {
            let data = comp.data.get_as::<ButtonData>().unwrap();
            data.state.replace(ButtonState::Pressed);
            Caribou::request_redraw();
            Caribou::instance().focused_component.set(Rc::downgrade(&comp));
        }));
        comp.on_primary_up.subscribe(Box::new(|comp| {
            let data = comp.data.get_as::<ButtonData>().unwrap();
            data.state.replace(ButtonState::Hover);
            let enabled = *data.enabled.get();
            if enabled {
                comp.action.broadcast(Rc::new(()));
            }
            Caribou::request_redraw();
        }));
        comp.on_mouse_enter.subscribe(Box::new(|comp| {
            let data = comp.data.get_as::<ButtonData>().unwrap();
            data.state.replace(ButtonState::Hover);
            Caribou::request_redraw();
        }));
        comp.on_mouse_leave.subscribe(Box::new(|comp| {
            let data = comp.data.get_as::<ButtonData>().unwrap();
            data.state.replace(ButtonState::Normal);
            Caribou::request_redraw();
        }));
        comp.size.set((100.0, 30.0).into());
        comp.data.set(Some(Box::new(ButtonData {
            text: comp.init_property(Rc::new("Button".to_string())),
            draw_normal: comp.init_event(),
            draw_hover: comp.init_event(),
            draw_pressed: comp.init_event(),
            draw_disabled: comp.init_event(),
            enabled: comp.init_property(true),
            state: RefCell::new(ButtonState::Normal),
            focused: RefCell::new(false)
        })));
        comp.on_gain_focus.subscribe(Box::new(|comp| {
            let data = comp.data.get_as::<ButtonData>().unwrap();
            if *data.enabled.get() {
                data.focused.replace(true);
                Caribou::request_redraw();
                println!("Gained focus!");
                true
            } else {
                false
            }
        }));
        comp.on_lose_focus.subscribe(Box::new(|comp| {
            println!("Lost focus!");
            let data = comp.data.get_as::<ButtonData>().unwrap();
            data.focused.replace(false);
            Caribou::request_redraw();
            true
        }));
        comp.on_key_down.subscribe(Box::new(|comp, event| {
            let data = comp.data.get_as::<ButtonData>().unwrap();
            match event.key {
                Key::Return | Key::Space | Key::NumpadEnter => {
                    data.state.replace(ButtonState::Pressed);
                    Caribou::request_redraw();
                }
                _ => {}
            }
        }));
        comp.on_key_up.subscribe(Box::new(|comp, event| {
            let data = comp.data.get_as::<ButtonData>().unwrap();
            match event.key {
                Key::Return | Key::Space | Key::NumpadEnter => {
                    data.state.replace(ButtonState::Normal);
                    comp.action.broadcast(Rc::new(()));
                    Caribou::request_redraw();
                }
                _ => {}
            }
        }));
        Caribou::register_auto_tab_order(&comp);
        comp
    }

    pub fn interpret(comp: &Rc<Component>) -> Option<Ref<ButtonData>> {
        comp.data.get_as::<ButtonData>()
    }
}

fn button_default_style_on_draw(
    border_mat: Material, back_mat: Material, caption_mat: Material
) -> Box<dyn Fn(Rc<Component>) -> Batch> {
    Box::new(move |comp| {
        let mut batch = Batch::new();
        let data = comp.data.get_as::<ButtonData>().unwrap();
        batch.add(BatchOp::Path {
            transform: Transform::default(),
            path: Path::from_vec(vec![
                PathOp::Rect((1.0, 1.0).into(),
                             *comp.size.get() - (2.0, 2.0).into()),

            ]),
            brush: Brush {
                stroke_mat: border_mat,
                fill_mat: back_mat,
                stroke_width: 2.0
            }
        });
        if *data.focused.borrow() {
            batch.add(BatchOp::Path {
                transform: Transform::default(),
                path: Path::from_vec(vec![
                    PathOp::Rect((1.0, 1.0).into(),
                                 *comp.size.get() - (2.0, 2.0).into()),
                ]),
                brush: Brush {
                    stroke_mat: Material::Solid(0.0, 0.0, 0.0, 1.0),
                    fill_mat: Material::Transparent,
                    stroke_width: 2.0
                }
            });
        }
        batch.add(BatchOp::Text {
            transform: Transform {
                translate: comp.size.get().times(0.5),
                ..Transform::default()
            },
            text: data.text.get().clone(),
            font: Font {
                family: Rc::new("Arial".to_string()),
                size: 16.0,
                weight: 500,
                slant: FontSlant::Normal,
            },
            alignment: TextAlignment::Center,
            brush: Brush {
                stroke_mat: Material::Transparent,
                fill_mat: caption_mat,
                stroke_width: 1.0
            }
        });
        batch
    })
}

impl ButtonData {
    pub fn apply_default_style(&self) {
        self.draw_normal.subscribe(button_default_style_on_draw(
            Material::Solid(0.95, 0.95, 0.95, 1.0),
            Material::Solid(0.95, 0.95, 0.95, 1.0),
            Material::Solid(0.0, 0.0, 0.0, 1.0),
        ));
        self.draw_hover.subscribe(button_default_style_on_draw(
            Material::Solid(0.9, 0.9, 0.9, 1.0),
            Material::Solid(0.9, 0.9, 0.9, 1.0),
            Material::Solid(0.0, 0.0, 0.0, 1.0),
        ));
        self.draw_pressed.subscribe(button_default_style_on_draw(
            Material::Solid(0.3, 0.3, 0.3, 1.0),
            Material::Solid(0.3, 0.3, 0.3, 1.0),
            Material::Solid(1.0, 1.0, 1.0, 1.0),
        ));
        self.draw_disabled.subscribe(button_default_style_on_draw(
            Material::Solid(0.95, 0.95, 0.95, 1.0),
            Material::Solid(0.95, 0.95, 0.95, 1.0),
            Material::Solid(0.4, 0.4, 0.4, 1.0),
        ));
    }
}

pub struct TextField;

pub struct TextFieldData {
    pub text: Property<String>,
    pub enabled: Property<bool>,
    pub focused: RefCell<bool>,
    pub draw_unfocused: ZeroArgEvent<Batch>,
    pub draw_focused: ZeroArgEvent<Batch>,
    pub draw_disabled: ZeroArgEvent<Batch>,
    pre_edit: RefCell<Option<String>>,
}

impl TextField {
    pub fn create() -> Rc<Component> {
        let comp = Component::create();
        comp.on_draw.subscribe(Box::new(|comp| {
            let data = comp.data.get_as::<TextFieldData>().unwrap();
            if *data.focused.borrow() {
                data.draw_focused.broadcast().consolidate()
            } else {
                data.draw_unfocused.broadcast().consolidate()
            }
        }));
        comp.on_primary_down.subscribe(Box::new(|comp| {
            let data = comp.data.get_as::<TextFieldData>().unwrap();
            if *data.enabled.get() {
                Caribou::instance().focused_component.set(Rc::downgrade(&comp));
            }
        }));
        comp.on_gain_focus.subscribe(Box::new(|comp| {
            let data = comp.data.get_as::<TextFieldData>().unwrap();
            if *data.enabled.get() {
                *data.focused.borrow_mut() = true;
                Caribou::request_redraw();
                true
            } else {
                false
            }
        }));
        comp.on_lose_focus.subscribe(Box::new(|comp| {
            let data = comp.data.get_as::<TextFieldData>().unwrap();
            *data.focused.borrow_mut() = false;
            Caribou::request_redraw();
            true
        }));
        comp.size.set((160.0, 30.0).into());
        comp.data.set(Some(Box::new(TextFieldData {
            text: comp.init_property(String::new()),
            enabled: comp.init_property(true),
            focused: false.into(),
            draw_unfocused: comp.init_event(),
            draw_focused: comp.init_event(),
            draw_disabled: comp.init_event(),
            pre_edit: None.into(),
        })));
        comp
    }
}