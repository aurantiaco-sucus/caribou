use std::any::Any;
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;
use log::info;
use widget::WidgetInner;
use event::{EventInit, SingleArgEvent};
use property::{Property, PropertyInit};

use crate::caribou::math::{IntPair, ScalarPair};
use crate::caribou::widgets::Layout;
use crate::caribou::input::{Key, KeyEvent};
use crate::caribou::widget::{create_widget, Widget, WidgetRef};

pub mod skia;

pub mod math;
pub mod batch;
pub mod widgets;
pub mod input;
pub mod window;
pub mod widget;
pub mod event;
pub mod property;
pub mod dispatch;

thread_local! {
    static ROOT_COMPONENT: RefCell<Widget> = Layout::create().into();
    static INSTANCE: Rc<Instance> = Rc::new(Instance::new());
}

pub struct Caribou;

impl Caribou {
    pub fn root_component() -> Widget {
        ROOT_COMPONENT.with(|root| root.borrow().clone())
    }

    pub fn replace_root_component(new_root: Widget) {
        ROOT_COMPONENT.with(|root| *root.borrow_mut() = new_root);
    }

    pub fn instance() -> Rc<Instance> {
        INSTANCE.with(|instance| instance.clone())
    }

    pub fn launch() {
        let instance = Caribou::instance();
        instance.on_key_down.subscribe(Box::new(|_, event| {
            if event.key == Key::Tab {
                Caribou::circulate_focus();
            } else if let Some(rc) =
            Caribou::instance().focused_component.get().upgrade() {
                rc.on_key_down.broadcast(event);
            }
        }));
        instance.on_key_up.subscribe(Box::new(|_, event| {
            if let Some(rc) =
            Caribou::instance().focused_component.get().upgrade() {
                rc.on_key_up.broadcast(event);
            }
        }));
        skia::runtime::skia_bootstrap();
    }

    pub fn request_redraw() {
        skia::skia_request_redraw();
    }

    pub fn register_auto_tab_order(rc: &Widget) {
        INSTANCE.with(|instance| {
            instance.auto_tab_order.borrow_mut().push(Rc::downgrade(rc));
        });
    }

    pub fn circulate_focus() -> bool {
        INSTANCE.with(|ins| {
            // Retain only valid components
            let mut manual = ins.manual_tab_order.borrow_mut();
            let mut auto = ins.auto_tab_order.borrow_mut();
            if !manual.is_empty() {
                manual.retain(|x| x.upgrade().is_some());
            }
            if !auto.is_empty() {
                auto.retain(|x| x.upgrade().is_some());
            }
            // Decide to use manual or auto
            let tab_order = if !manual.is_empty() {
                manual
            } else {
                auto
            };
            // Stop focusing if there is no component to do so
            if tab_order.is_empty() {
                ins.focused_component.reset();
                return true;
            }
            // Check if the current focused component is still valid
            let mut cur_ref = ins.focused_component.get_mut();
            let initial_next = if let Some(cur_now) = cur_ref.upgrade() {
                // Ask the current focused component to give up focus
                if cur_now.on_lose_focus.any_false() {
                    return false;
                }
                let cur_index = tab_order.iter()
                    .position(|x| Rc::ptr_eq(&cur_now,
                                             &x.upgrade().unwrap()));
                cur_index.map(|x| (x + 1) % tab_order.len()).unwrap_or(0)
            } else { 0 };
            let mut next_index = initial_next;
            // Circularly find the next component to focus
            loop {
                let next = tab_order[next_index].upgrade().unwrap();
                // Ask the next component to take focus
                if next.on_gain_focus.none_false() {
                    println!("Focus on #{}", next_index);
                    *cur_ref = tab_order[next_index].clone();
                    return true;
                }
                next_index = (next_index + 1) % tab_order.len();
                // If we have tried all components, stop focusing
                if next_index == initial_next {
                    return false;
                }
            }
        })
    }
}

pub struct Instance {
    placeholder: Widget,
    pub manual_tab_order: RefCell<Vec<WidgetRef>>,
    pub auto_tab_order: RefCell<Vec<WidgetRef>>,
    pub focused_component: Property<WidgetRef>,
    pub on_key_down: SingleArgEvent<KeyEvent>,
    pub on_key_up: SingleArgEvent<KeyEvent>,
}

impl Instance {
    fn new() -> Self {
        let dummy = create_widget();
        Self {
            placeholder: dummy.clone(),
            manual_tab_order: RefCell::new(vec![]),
            auto_tab_order: RefCell::new(vec![]),
            focused_component: dummy.init_default_property(),
            on_key_down: dummy.init_event(),
            on_key_up: dummy.init_event(),
        }
    }
}