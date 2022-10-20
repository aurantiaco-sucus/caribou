#![feature(const_btree_new)]
#![feature(unchecked_math)]

use crate::caribou::{Caribou, Component};
use crate::caribou::basic::{Button, Layout};

mod caribou;

fn main() {
    let root = Caribou::root_component();
    let layout = Layout::interpret(&root).unwrap();
    let button1 = Button::create();
    Button::interpret(&button1).unwrap().apply_default_style();
    let button2 = Button::create();
    button2.position.set((50.0, 20.0).into());
    Button::interpret(&button2).unwrap().apply_default_style();
    layout.children.borrow_mut().push(button1);
    layout.children.borrow_mut().push(button2);
    root.size.set((640.0, 400.0).into());
    Caribou::launch();
}
