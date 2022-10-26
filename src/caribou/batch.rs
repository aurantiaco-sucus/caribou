use std::any::Any;
use std::cell::{Ref, RefCell};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::{Arc, LockResult, Mutex, MutexGuard, RwLock, RwLockReadGuard};
use crate::caribou::math::ScalarPair;

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Batch {
    data: Arc<RwLock<Vec<BatchOp>>>,
}

impl Batch {
    pub fn new() -> Batch {
        Batch { data: Arc::new(Vec::new().into()) }
    }

    pub fn add_op(&self, op: BatchOp) {
        self.data.write().unwrap().push(op);
    }

    pub fn append(&self, other: Batch) {
        self.data.write().unwrap().extend(other.data.read().unwrap().clone());
    }

    pub fn data(&self) -> LockResult<RwLockReadGuard<Vec<BatchOp>>> {
        self.data.read()
    }
}

pub trait BatchConsolidation {
    fn consolidate(self) -> Batch;
}

impl BatchConsolidation for Vec<Batch> {
    fn consolidate(self) -> Batch {
        let mut batch = Batch::new();
        for entry in self {
            batch.append(entry);
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
        text: String,
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

pub trait PictImpl: Send + Sync + Debug {
    fn get(&self) -> Box<dyn Any>;
}

#[derive(Debug, Clone)]
pub struct Pict {
    data: Arc<RwLock<Box<dyn PictImpl>>>,
}

impl Pict {
    pub fn new(data: Box<dyn PictImpl>) -> Pict {
        Pict { data: Arc::new(RwLock::new(data)) }
    }

    pub fn data(&self) -> LockResult<RwLockReadGuard<Box<dyn PictImpl>>> {
        self.data.read()
    }
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Path {
    data: Arc<RwLock<Vec<PathOp>>>
}

impl Path {
    pub fn new() -> Path {
        Path { data: Arc::new(Vec::new().into()) }
    }
    
    pub fn from_vec(data: Vec<PathOp>) -> Path {
        Path { data: Arc::new(data.into()) }
    }

    pub fn add(&mut self, op: PathOp) {
        self.data.write().unwrap().push(op);
    }

    pub fn add_path(&mut self, path: Path) {
        self.data.write().unwrap().extend(path.data.write().unwrap().clone());
    }

    pub fn data(&self) -> LockResult<RwLockReadGuard<Vec<PathOp>>> {
        self.data.read()
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

impl Default for Brush {
    fn default() -> Self {
        Brush::transparent()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Material {
    Transparent,
    Solid(f32, f32, f32, f32),
}

#[derive(Debug, Clone)]
pub struct Font {
    pub family: Arc<String>,
    pub size: f32,
    pub weight: i32,
    pub slant: FontSlant,
}

impl Default for Font {
    fn default() -> Self {
        Font {
            family: Arc::new("DengXian".to_string()),
            size: 12.0,
            weight: 400,
            slant: FontSlant::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FontSlant {
    Normal,
    Italic,
    Oblique,
}