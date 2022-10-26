use std::any::Any;
use std::iter::FilterMap;
use std::rc::{Rc, Weak};
use std::slice::Iter;
use crate::caribou::batch::{Batch, Brush, Font};
use crate::caribou::event::{EventInit, SingleArgEvent, ZeroArgEvent};
use crate::caribou::input::KeyEvent;
use crate::caribou::math::IntPair;
use crate::caribou::property::*;

pub type Widget = Rc<WidgetInner>;
pub type WidgetRef = Weak<WidgetInner>;

pub struct WidgetInner {
    // Attributes
    // - Generic
    pub position: ScalarProperty,
    pub size: ScalarProperty,
    pub enabled: BoolProperty,
    // - Hierarchical
    pub parent: OptionalProperty<WidgetRef>,
    pub content: OptionalProperty<Widget>,
    pub children: VecProperty<Widget>,
    // - Appearance
    pub background: Property<Brush>,
    pub foreground: Property<Brush>,
    pub boarder: Property<Brush>,
    pub font: Property<Font>,
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

pub fn create_widget() -> Widget {
    Rc::new_cyclic(|back| {
        WidgetInner {
            position: back.init_default_property(),
            size: back.init_default_property(),
            enabled: back.init_property(true),
            parent: back.init_default_property(),
            content: back.init_default_property(),
            children: back.init_default_property(),
            background: back.init_default_property(),
            foreground: back.init_default_property(),
            boarder: back.init_default_property(),
            font: back.init_default_property(),
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
        }
    })
}

trait SameAs {
    fn same_as(&self, other: &Self) -> bool;
}

impl SameAs for Widget {
    fn same_as(&self, other: &Self) -> bool {
        Rc::ptr_eq(self, other)
    }
}

impl SameAs for WidgetRef {
    fn same_as(&self, other: &Self) -> bool {
        Weak::ptr_eq(self, other)
    }
}

pub trait WidgetRefer {
    fn refer(&self) -> WidgetRef;
}

impl WidgetRefer for Widget {
    fn refer(&self) -> WidgetRef {
        Rc::downgrade(&self).into()
    }
}

pub trait WidgetAcquire {
    fn acquire(&self) -> Option<Widget>;
}

impl WidgetAcquire for WidgetRef {
    fn acquire(&self) -> Option<Widget> {
        self.upgrade().map(|inner| inner.into())
    }
}

pub trait WidgetRefVec {
    fn clean(&mut self);
    fn acquire(&self) -> FilterMap<Iter<WidgetRef>, fn(&WidgetRef) -> Option<Widget>>;
    fn contains_widget(&self, widget: &Widget) -> bool;
    fn contains_ref(&self, widget: &WidgetRef) -> bool;
}

impl WidgetRefVec for Vec<WidgetRef> {
    fn clean(&mut self) {
        self.retain(|item| item.upgrade().is_some());
    }

    fn acquire(&self) -> FilterMap<Iter<WidgetRef>, fn(&WidgetRef) -> Option<Widget>> {
        self.iter().filter_map(|x| x.upgrade())
    }

    fn contains_widget(&self, widget: &Widget) -> bool {
        self.iter()
            .any(|x| match x.upgrade() {
                None => false,
                Some(other) => widget.same_as(&other),
            })
    }

    fn contains_ref(&self, widget: &WidgetRef) -> bool {
        self.iter()
            .any(|x| x.same_as(widget))
    }
}

pub trait WidgetVec {
    fn contains_widget(&self, widget: &Widget) -> bool;
}

impl WidgetVec for Vec<Widget> {
    fn contains_widget(&self, widget: &Widget) -> bool {
        self.iter()
            .any(|x| widget.same_as(x))
    }
}