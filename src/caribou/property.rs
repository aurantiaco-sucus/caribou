use std::cell::{Ref, RefCell, RefMut};
use std::any::Any;
use std::ops::{Add, AddAssign, Deref};
use std::rc::{Rc, Weak};
use crate::caribou::math::{IntPair, ScalarPair};
use crate::caribou::widget::{Widget, WidgetRef};
use crate::WidgetInner;

pub type ScalarProperty = Property<ScalarPair>;
pub type IntProperty = Property<IntPair>;
pub type BoolProperty = Property<bool>;
pub type OptionalProperty<T> = Property<Option<T>>;
pub type VecProperty<T> = Property<Vec<T>>;

pub type DynamicProperty = OptionalProperty<Box<dyn Any>>;

impl<T> Property<T> where T: Add<Output=T>, T: Copy {
    pub fn offset_by(&self, offset: T) {
        self.set(self.get().add(offset));
    }
}

impl<T> Property<T> where T: Default {
    pub fn reset(&self) {
        self.set(T::default());
    }
}

impl BoolProperty {
    pub fn flip(&self) {
        self.set(!*self.get());
    }

    pub fn is_true(&self) -> bool {
        *self.get()
    }

    pub fn is_false(&self) -> bool {
        !*self.get()
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

impl<T> VecProperty<T> {
    pub fn push(&self, value: T) {
        self.value.borrow_mut().push(value);
        self.inform();
    }

    pub fn pop(&self) -> Option<T> {
        let value = self.value.borrow_mut().pop();
        self.inform();
        value
    }

    pub fn remove(&self, index: usize) -> T {
        let value = self.value.borrow_mut().remove(index);
        self.inform();
        value
    }

    pub fn insert(&self, index: usize, value: T) {
        self.value.borrow_mut().insert(index, value);
        self.inform();
    }

    pub fn clear(&self) {
        self.value.borrow_mut().clear();
        self.inform();
    }
}

impl DynamicProperty {
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

pub trait PropertyInit<T> {
    fn init_property(&self, initial: T) -> Property<T>;
    fn init_default_property(&self) -> Property<T> where T: Default {
        self.init_property(T::default())
    }
}

impl<T> PropertyInit<T> for WidgetRef {
    fn init_property(&self, initial: T) -> Property<T> {
        Property::new(initial, self.clone())
    }
}

impl<T> PropertyInit<T> for Widget {
    fn init_property(&self, initial: T) -> Property<T> {
        Property::new(initial, Rc::downgrade(self))
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
    back_ref: WidgetRef,
}

impl<T> Property<T> {
    pub fn new(initial: T, back_ref: WidgetRef) -> Property<T> {
        Property {
            value: RefCell::new(initial).into(),
            listeners: RefCell::new(Vec::new()).into(),
            back_ref,
        }
    }

    pub fn get(&self) -> Ref<T> {
        self.value.borrow()
    }

    pub fn get_cloned(&self) -> T where T: Clone {
        self.value.borrow().clone()
    }

    pub fn get_copy(&self) -> T where T: Copy {
        *self.value.borrow()
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
