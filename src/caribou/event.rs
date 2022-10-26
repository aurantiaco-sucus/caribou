use std::cell::RefCell;
use std::rc::{Rc, Weak};
use crate::caribou::widget::{Widget, WidgetRef};
use crate::WidgetInner;

pub type ZeroArgEvent<R=()> = Event<Box<dyn Fn(Widget) -> R>>;
pub type SingleArgEvent<A, R=()> = Event<Box<dyn Fn(Widget, A) -> R>>;

pub trait EventInit<T> {
    fn init_event(&self) -> Event<T>;
}

impl<T> EventInit<T> for WidgetRef {
    fn init_event(&self) -> Event<T> {
        Event::new(self.clone())
    }
}

impl<T> EventInit<T> for Widget {
    fn init_event(&self) -> Event<T> {
        Event::new(Rc::downgrade(self))
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
    back_ref: WidgetRef,
}

impl<F> Event<F> {
    pub fn new(back_ref: WidgetRef) -> Self {
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

impl<R> Event<Box<dyn Fn(Widget) -> R>> {
    pub fn broadcast(&self) -> Vec<R> {
        let mut results = Vec::new();
        for listener in self.listeners.borrow().iter() {
            results.push((listener.func)(self.back_ref.upgrade().unwrap()));
        }
        results
    }
}

impl<T, R> Event<Box<dyn Fn(Widget, T) -> R>> where T: Clone {
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
