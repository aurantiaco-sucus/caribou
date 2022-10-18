use std::any::Any;
use std::cell::{Ref, RefCell};
use std::rc::Rc;
use crate::caribou::math::ScalarPair;

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Batch {
    data: Rc<RefCell<Vec<BatchOp>>>,
}

impl Batch {
    pub fn new() -> Batch {
        Batch {
            data: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn add(&mut self, op: BatchOp) {
        self.data.borrow_mut().push(op);
    }

    pub fn add_batch(&mut self, batch: Batch) {
        self.data.borrow_mut().append(&mut batch.data.borrow_mut());
    }

    pub fn iter(&self) -> BatchIter {
        BatchIter {
            data: self.data.clone(),
            index: 0,
        }
    }
}

pub struct BatchIter {
    data: Rc<RefCell<Vec<BatchOp>>>,
    index: usize,
}

impl Iterator for BatchIter {
    type Item = BatchOp;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        self.data.borrow().get(index).cloned()
    }
}

pub trait BatchConsolidation {
    fn consolidate(self) -> Batch;
}

impl BatchConsolidation for Vec<Batch> {
    fn consolidate(self) -> Batch {
        let mut batch = Batch::new();
        for entry in self {
            batch.add_batch(entry);
        }
        batch
    }
}

#[derive(Debug, Clone)]
pub enum BatchOp {
    Pict {
        transform: Transform,
        pict: Pict,
    },
    Path {
        transform: Transform,
        path: Path,
        brush: Brush,
    },
    Text {
        transform: Transform,
        text: Rc<String>,
        font: Font,
        alignment: TextAlignment,
        brush: Brush,
    },
    Batch {
        transform: Transform,
        batch: Batch,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub translate: ScalarPair,
    pub scale: ScalarPair,
    pub rotate: f32,
    pub rotate_center: ScalarPair,
    pub clip_size: Option<ScalarPair>,
}

impl Default for Transform {
    fn default() -> Self {
        Transform {
            translate: (0.0, 0.0).into(),
            scale: (1.0, 1.0).into(),
            rotate: 0.0,
            rotate_center: (0.0, 0.0).into(),
            clip_size: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TextAlignment {
    Origin,
    Center
}

#[derive(Debug, Clone)]
pub struct Pict {
    data: Rc<RefCell<Box<dyn Any>>>
}

impl Pict {
    pub fn encapsulate<T: Any>(data: T) -> Pict {
        Pict {
            data: Rc::new(RefCell::new(Box::new(data)))
        }
    }

    pub fn downcast<T: Any>(&self) -> Option<Ref<T>> {
        match Ref::filter_map(self.data.borrow(),
                              |x| x.downcast_ref::<T>())
        { Ok(val) => Some(val), Err(_) => None }
    }
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Path {
    data: Rc<RefCell<Vec<PathOp>>>
}

impl Path {
    pub fn new() -> Path {
        Path {
            data: Rc::new(RefCell::new(Vec::new())),
        }
    }
    
    pub fn from_vec(data: Vec<PathOp>) -> Path {
        Path {
            data: Rc::new(RefCell::new(data)),
        }
    }

    pub fn add(&mut self, op: PathOp) {
        self.data.borrow_mut().push(op);
    }

    pub fn add_path(&mut self, path: Path) {
        self.data.borrow_mut().append(&mut path.data.borrow_mut());
    }

    pub fn iter(&self) -> PathIter {
        PathIter {
            data: self.data.clone(),
            index: 0,
        }
    }
}

pub struct PathIter {
    data: Rc<RefCell<Vec<PathOp>>>,
    index: usize,
}

impl Iterator for PathIter {
    type Item = PathOp;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        self.data.borrow().get(index).cloned()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PathOp {
    MoveTo(ScalarPair),
    LineTo(ScalarPair),
    QuadTo(ScalarPair, ScalarPair),
    CubicTo(ScalarPair, ScalarPair, ScalarPair),
    Close,
    Line(ScalarPair, ScalarPair),
    Rect(ScalarPair, ScalarPair),
    Oval(ScalarPair, ScalarPair),
}

#[derive(Debug, Clone, Copy)]
pub struct Brush {
    pub stroke_mat: Material,
    pub fill_mat: Material,
    pub stroke_width: f32,
}

impl Brush {
    pub fn solid_stroke(mat: Material, stroke_width: f32) -> Brush {
        Brush {
            stroke_mat: mat,
            fill_mat: Material::Transparent,
            stroke_width,
        }
    }

    pub fn solid_fill(mat: Material) -> Brush {
        Brush {
            stroke_mat: Material::Transparent,
            fill_mat: mat,
            stroke_width: 0.0,
        }
    }

    pub fn transparent() -> Brush {
        Brush {
            stroke_mat: Material::Transparent,
            fill_mat: Material::Transparent,
            stroke_width: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Material {
    Transparent,
    Solid(f32, f32, f32, f32),
}

#[derive(Debug, Clone)]
pub struct Font {
    pub family: Rc<String>,
    pub size: f32,
    pub weight: i32,
    pub slant: FontSlant,
}

#[derive(Debug, Clone, Copy)]
pub enum FontSlant {
    Normal,
    Italic,
    Oblique,
}