use std::any::Any;
use std::cell::{Ref, RefCell};
use std::rc::{Rc, Weak};
use crate::caribou::draw::{Batch};
use crate::caribou::math::{IntPair, ScalarPair};

pub mod skia;

pub mod math;
pub mod draw;
pub mod basic;

pub struct Component {
    // Standard attributes
    position: Property<ScalarPair>,
    size: Property<ScalarPair>,
    needs_redraw: Property<bool>,
    // Arbitrary attribute
    data: Property<Option<Box<dyn Any>>>,
    // Activation event
    action: Event<Box<dyn Fn(Rc<Component>, Rc<dyn Any>)>>,
    // Standard events
    on_draw: Event<Box<dyn Fn(Rc<Component>) -> Batch>>,
    on_update: Event<Box<dyn Fn(Rc<Component>)>>,
    on_mouse_down: Event<Box<dyn Fn(Rc<Component>)>>,
    on_mouse_up: Event<Box<dyn Fn(Rc<Component>)>>,
    on_mouse_move: Event<Box<dyn Fn(Rc<Component>, IntPair)>>,
    on_mouse_enter: Event<Box<dyn Fn(Rc<Component>)>>,
    on_mouse_leave: Event<Box<dyn Fn(Rc<Component>)>>,
}

impl Component {
    pub fn create() -> Rc<Component> {
        Rc::new_cyclic(|weak| {
            let comp = Component {
                position: Property::new(ScalarPair::new(0.0, 0.0), weak.clone()),
                size: Property::new(ScalarPair::new(0.0, 0.0), weak.clone()),
                needs_redraw: Property::new(true, weak.clone()),
                data: Property::new(None, weak.clone()),
                action: Event::new(weak.clone()),
                on_draw: Event::new(weak.clone()),
                on_update: Event::new(weak.clone()),
                on_mouse_down: Event::new(weak.clone()),
                on_mouse_up: Event::new(weak.clone()),
                on_mouse_move: Event::new(weak.clone()),
                on_mouse_enter: Event::new(weak.clone()),
                on_mouse_leave: Event::new(weak.clone()),
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

    pub fn set(&self, value: T) {
        for listener in self.listeners.borrow().iter() {
            listener.invoke(&value);
        }
        *self.value.borrow_mut() = value;
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