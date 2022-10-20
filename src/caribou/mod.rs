use std::any::Any;
use std::cell::{Ref, RefCell, RefMut};
use std::rc::{Rc, Weak};
use log::info;
use crate::caribou::draw::{Batch};
use crate::caribou::math::{IntPair, ScalarPair};
use crate::caribou::basic::Layout;
use crate::caribou::input::{Key, KeyEvent};

pub mod skia;

pub mod math;
pub mod draw;
pub mod basic;
pub mod input;
pub mod window;
pub mod component;

thread_local! {
    static ROOT_COMPONENT: RefCell<Rc<Component>> = Layout::create().into();
    static INSTANCE: Rc<Instance> = Rc::new(Instance::new());
}

pub struct Caribou;

impl Caribou {
    pub fn root_component() -> Rc<Component> {
        ROOT_COMPONENT.with(|root| root.borrow().clone())
    }

    pub fn replace_root_component(new_root: Rc<Component>) {
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
        skia::draw::skia_request_redraw();
    }

    pub fn register_auto_tab_order(rc: &Rc<Component>) {
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
                ins.focused_component.set_default();
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

pub type ScalarProperty = Property<ScalarPair>;
pub type IntProperty = Property<IntPair>;
pub type OptionalProperty<T> = Property<Option<T>>;
pub type DynamicProperty = OptionalProperty<Box<dyn Any>>;

pub type ZeroArgEvent<R=()> = Event<Box<dyn Fn(Rc<Component>) -> R>>;
pub type SingleArgEvent<A, R=()> = Event<Box<dyn Fn(Rc<Component>, A) -> R>>;

pub struct Instance {
    placeholder: Rc<Component>,
    pub manual_tab_order: RefCell<Vec<Weak<Component>>>,
    pub auto_tab_order: RefCell<Vec<Weak<Component>>>,
    pub focused_component: Property<Weak<Component>>,
    pub on_key_down: SingleArgEvent<KeyEvent>,
    pub on_key_up: SingleArgEvent<KeyEvent>,
}

impl Instance {
    fn new() -> Self {
        let dummy = Component::create();
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

pub struct Component {
    // Attributes
    // - Standard
    pub position: ScalarProperty,
    pub size: ScalarProperty,
    pub needs_redraw: Property<bool>,
    // - Arbitrary
    pub data: DynamicProperty,
    // Events
    // - Action
    pub action: SingleArgEvent<Rc<dyn Any>>,
    // - Render & update
    pub on_draw: ZeroArgEvent<Batch>,
    pub on_update: ZeroArgEvent,
    // - Mouse
    // -- Button
    pub on_primary_down: ZeroArgEvent,
    pub on_primary_up: ZeroArgEvent,
    pub on_secondary_down: ZeroArgEvent,
    pub on_secondary_up: ZeroArgEvent,
    pub on_tertiary_down: ZeroArgEvent,
    pub on_tertiary_up: ZeroArgEvent,
    // -- Motion
    pub on_mouse_move: SingleArgEvent<IntPair>,
    pub on_mouse_enter: ZeroArgEvent,
    pub on_mouse_leave: ZeroArgEvent,
    // - Focus
    // -- Generic
    pub on_gain_focus: ZeroArgEvent<bool>,
    pub on_lose_focus: ZeroArgEvent<bool>,
    // -- Keyboard
    pub on_key_down: SingleArgEvent<KeyEvent>,
    pub on_key_up: SingleArgEvent<KeyEvent>,
    // -- Input
    pub on_pre_edit: SingleArgEvent<String>,
    pub on_commit: SingleArgEvent<String>,
}

pub trait PropertyInit<T> {
    fn init_property(&self, initial: T) -> Property<T>;
    fn init_default_property(&self) -> Property<T> where T: Default {
        self.init_property(T::default())
    }
}

impl<T> PropertyInit<T> for Weak<Component> {
    fn init_property(&self, initial: T) -> Property<T> {
        Property::new(initial, self.clone())
    }
}

impl<T> PropertyInit<T> for Rc<Component> {
    fn init_property(&self, initial: T) -> Property<T> {
        Property::new(initial, Rc::downgrade(self))
    }
}

pub trait EventInit<T> {
    fn init_event(&self) -> Event<T>;
}

impl<T> EventInit<T> for Weak<Component> {
    fn init_event(&self) -> Event<T> {
        Event::new(self.clone())
    }
}

impl<T> EventInit<T> for Rc<Component> {
    fn init_event(&self) -> Event<T> {
        Event::new(Rc::downgrade(self))
    }
}

impl Component {
    pub fn create() -> Rc<Component> {
        Rc::new_cyclic(|back| {
            let comp = Component {
                position: back.init_default_property(),
                size: back.init_default_property(),
                needs_redraw: back.init_property(true),
                data: back.init_default_property(),
                action: back.init_event(),
                on_draw: back.init_event(),
                on_update: back.init_event(),
                on_primary_down: back.init_event(),
                on_primary_up: back.init_event(),
                on_secondary_down: back.init_event(),
                on_secondary_up: back.init_event(),
                on_tertiary_down: back.init_event(),
                on_tertiary_up: back.init_event(),
                on_mouse_move: back.init_event(),
                on_mouse_enter: back.init_event(),
                on_mouse_leave: back.init_event(),
                on_gain_focus: back.init_event(),
                on_lose_focus: back.init_event(),
                on_key_down: back.init_event(),
                on_key_up: back.init_event(),
                on_pre_edit: back.init_event(),
                on_commit: back.init_event(),
            };
            comp
        })
    }
}

pub trait ComponentRcVec {
    fn comp_contains(&self, comp: &Rc<Component>) -> bool;
}

impl ComponentRcVec for Vec<Rc<Component>> {
    fn comp_contains(&self, comp: &Rc<Component>) -> bool {
        self.iter().any(|x| Rc::ptr_eq(x, comp))
    }
}

type ListenerFunc<T> = Box<dyn Fn(&T)>;

pub struct Listener<T> {
    func: Rc<ListenerFunc<T>>,
}

impl<T> Clone for Listener<T> {
    fn clone(&self) -> Self {
        Listener {
            func: self.func.clone(),
        }
    }
}

impl<T> Listener<T> {
    pub fn new(func: ListenerFunc<T>) -> Listener<T> {
        Listener {
            func: Rc::new(func),
        }
    }

    pub fn invoke(&self, value: &T) {
        (self.func)(value);
    }
}

impl<T> PartialEq for Listener<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.func, &other.func)
    }
}

#[derive(Clone)]
pub struct Property<T> {
    value: Rc<RefCell<T>>,
    listeners: Rc<RefCell<Vec<Listener<T>>>>,
    back_ref: Weak<Component>,
}

impl<T> Property<T> {
    pub fn new(initial: T, back_ref: Weak<Component>) -> Property<T> {
        Property {
            value: RefCell::new(initial).into(),
            listeners: RefCell::new(Vec::new()).into(),
            back_ref,
        }
    }

    pub fn get(&self) -> Ref<T> {
        self.value.borrow()
    }

    pub fn get_mut(&self) -> RefMut<T> {
        self.value.borrow_mut()
    }

    pub fn set(&self, value: T) {
        for listener in self.listeners.borrow().iter() {
            listener.invoke(&value);
        }
        *self.value.borrow_mut() = value;
    }
    
    pub fn inform(&self) {
        for listener in self.listeners.borrow().iter() {
            listener.invoke(&self.value.borrow());
        }
    }

    pub fn listen(&self, listener: Box<dyn Fn(&T)>) -> Listener<T> {
        let listener = Listener::new(listener);
        self.listeners.borrow_mut().push(listener.clone());
        listener
    }

    pub fn unlisten(&self, listener: &Listener<T>) {
        self.listeners.borrow_mut().retain(|l| l != listener);
    }
}

impl Property<Option<Box<dyn Any>>> {
    pub fn get_as<T: 'static>(&self) -> Option<Ref<T>> {
        match Ref::filter_map(
            self.get(), |a|
                a.as_ref()?.as_ref().downcast_ref::<T>())
        {
            Ok(val) => Some(val),
            Err(_) => None
        }
    }
}

pub struct Subscriber<F> {
    func: Rc<F>,
}

impl<F> Clone for Subscriber<F> {
    fn clone(&self) -> Self {
        Subscriber {
            func: self.func.clone(),
        }
    }
}

impl<F> Subscriber<F> {
    pub fn new(func: F) -> Subscriber<F> {
        Subscriber {
            func: Rc::new(func),
        }
    }
}

impl<T> PartialEq for Subscriber<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.func, &other.func)
    }
}

impl<T> OptionalProperty<T> {
    pub fn is_some(&self) -> bool {
        self.value.borrow().is_some()
    }

    pub fn put(&self, value: T) {
        self.value.replace(Some(value));
        for listener in self.listeners.borrow().iter() {
            listener.invoke(&self.value.borrow());
        }
    }

    pub fn take(&self) -> Option<T> {
        let value = self.value.borrow_mut().take();
        for listener in self.listeners.borrow().iter() {
            listener.invoke(&self.value.borrow());
        }
        value
    }

    pub fn clear(&self) {
        self.value.replace(None);
        for listener in self.listeners.borrow().iter() {
            listener.invoke(&self.value.borrow());
        }
    }
}

impl<T> Property<T> where T: Default {
    pub fn set_default(&self) {
        self.set(T::default());
    }
}

pub struct Event<F> {
    listeners: RefCell<Vec<Subscriber<F>>>,
    back_ref: Weak<Component>,
}

impl<F> Event<F> {
    pub fn new(back_ref: Weak<Component>) -> Self {
        Self {
            listeners: RefCell::new(Vec::new()),
            back_ref,
        }
    }

    pub fn subscribe(&self, listener: F) -> Subscriber<F> {
        let func = Subscriber::new(listener);
        self.listeners.borrow_mut().push(func.clone());
        func
    }

    pub fn unsubscribe(&self, listener: Subscriber<F>) {
        let mut listeners = self.listeners.borrow_mut();
        let index = listeners.iter().position(|l| l == &listener);
        if let Some(index) = index {
            listeners.swap_remove(index);
        }
    }
}

impl<R> Event<Box<dyn Fn(Rc<Component>) -> R>> {
    pub fn broadcast(&self) -> Vec<R> {
        let mut results = Vec::new();
        for listener in self.listeners.borrow().iter() {
            results.push((listener.func)(self.back_ref.upgrade().unwrap()));
        }
        results
    }
}

impl<T, R> Event<Box<dyn Fn(Rc<Component>, T) -> R>> where T: Clone {
    pub fn broadcast(&self, value: T) -> Vec<R> {
        let mut results = Vec::new();
        for listener in self.listeners.borrow().iter() {
            results.push((listener.func)(self.back_ref.upgrade().unwrap(), value.clone()));
        }
        results
    }
}

impl ZeroArgEvent<bool> {
    pub fn none_true(&self) -> bool {
        !self.broadcast().iter().any(|x| *x)
    }

    pub fn none_false(&self) -> bool {
        !self.broadcast().iter().any(|x| !*x)
    }

    pub fn any_true(&self) -> bool {
        self.broadcast().iter().any(|x| *x)
    }

    pub fn any_false(&self) -> bool {
        self.broadcast().iter().any(|x| !*x)
    }
}