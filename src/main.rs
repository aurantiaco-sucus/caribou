#![feature(const_btree_new)]
#![feature(unchecked_math)]

use crate::caribou::Caribou;
use crate::caribou::widgets::{Button, Layout};
use self::caribou::widget::WidgetInner;

mod caribou;

fn main() {
    let root = Caribou::root_component();
    let button1 = Button::create();
    Button::interpret(&button1).unwrap().apply_default_style();
    let button2 = Button::create();
    button2.position.set((50.0, 20.0).into());
    Button::interpret(&button2).unwrap().apply_default_style();
    root.children.push(button1);
    root.children.push(button2);
    root.size.set((640.0, 400.0).into());
    Caribou::launch();
}
