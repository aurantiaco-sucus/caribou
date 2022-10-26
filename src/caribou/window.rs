use std::collections::{LinkedList, VecDeque};
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{Builder, JoinHandle, spawn, Thread};
use crate::caribou::property::{IntProperty, Property, PropertyInit, ScalarProperty};
use crate::{Layout, WidgetInner};
use crate::caribou::batch::{Batch};
use crate::caribou::skia::runtime::skia_bootstrap;
use crate::caribou::widget::{create_widget, Widget};

pub struct Window {
    marker: Widget,
    pub title: Property<String>,
    pub size: IntProperty,
    pub root: Property<Widget>,
}

unsafe impl Send for Window {}

impl Window {
    pub fn new() -> Window {
        let marker = create_widget();
        Window {
            marker: marker.clone(),
            title: marker.init_default_property(),
            size: marker.init_default_property(),
            root: marker.init_property(create_widget()),
        }
    }
}

pub struct Handshake {
    dispatch_queue: Mutex<LinkedList<DispatchMessage>>,
    backend_queue: Mutex<LinkedList<BackendMessage>>,
}

impl Handshake {
    pub fn create() -> Arc<Handshake> {
        Arc::new(Handshake {
            dispatch_queue: Default::default(),
            backend_queue: Default::default()
        })
    }
}

pub enum DispatchMessage {
    BackendInitialized,
    RequestRedraw,
}

pub enum BackendMessage {
    PerformRedraw(Batch),
}

pub fn launch_blocking(window: Window) {
    let handshake = Handshake::create();
    let handshake_dispatch = handshake.clone();
    let dispatch_thread = spawn(move || {
        let window = window;
        let handshake = handshake_dispatch;
    });
    let handshake_backend = handshake;
    let backend_thread = spawn(move || {
        let handshake = handshake_backend;
    });
    dispatch_thread.join().unwrap();
    backend_thread.join().unwrap();
}