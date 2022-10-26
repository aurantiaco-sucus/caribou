use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::{Duration, Instant};

pub type Task = Box<dyn FnOnce() + Send>;

pub struct Dispatcher {
    deque: Mutex<VecDeque<Task>>,
    notifier: Condvar,
    states: Vec<Arc<AtomicBool>>,
}

static mut DISPATCHER: Option<Dispatcher> = None;

impl Dispatcher {
    pub fn launch() {
        unsafe {
            DISPATCHER = Some(Self {
                deque: Mutex::new(VecDeque::new()),
                notifier: Condvar::new(),
                states: vec![]
            });
        }
        let thread_count = thread::available_parallelism().unwrap().get();
        let instance = Self::instance_mut();
        for _ in 0..thread_count {
            let state = Arc::new(AtomicBool::new(true));
            instance.states.push(state.clone());
            thread::spawn(move || {
                // Notice that the (updated) state won't be consumed when it's waiting for a task
                while state.load(std::sync::atomic::Ordering::Relaxed) {
                    let task = Self::pop();
                    task();
                }
            });
        }
    }

    fn instance() -> &'static Dispatcher {
        unsafe { DISPATCHER.as_ref().unwrap() }
    }

    fn instance_mut() -> &'static mut Dispatcher {
        unsafe { DISPATCHER.as_mut().unwrap() }
    }

    pub fn push(func: Box<dyn FnOnce() + Send>) {
        let mut deque = Dispatcher::instance().deque.lock().unwrap();
        deque.push_back(func);
        drop(deque);
        // Inform a thread to take a task
        Dispatcher::instance().notifier.notify_one();
    }

    pub fn pop() -> Box<dyn FnOnce() + Send> {
        let instance = Dispatcher::instance_mut();
        let mut deque = instance.deque.lock().unwrap();
        // Wait until the queue is not empty
        while deque.is_empty() {
            deque = instance.notifier.wait(deque).unwrap();
        }
        let task = deque.pop_front().unwrap();
        // Inform another thread to take a task if there are any more
        if !deque.is_empty() {
            instance.notifier.notify_one();
        }
        task
    }

    pub fn shutdown() {
        // Inform all threads to stop
        for state in &Dispatcher::instance().states {
            state.store(false, std::sync::atomic::Ordering::Relaxed);
        }
        // Ensure all threads have a chance to break out of the loop
        for _ in 0..Dispatcher::instance().states.len() {
            Dispatcher::push(Box::new(|| {}));
        }
    }
}

pub struct DelayedTask {
    task: Task,
    deploy_instant: Instant,
    delay: Duration,
}

impl DelayedTask {
    pub fn new(task: Task, delay: Duration) -> Self {
        Self {
            task,
            deploy_instant: Instant::now(),
            delay,
        }
    }

    pub fn is_ready(&self) -> bool {
        Instant::now() - self.deploy_instant >= self.delay
    }
}

pub struct Scheduler {
    tasks: Mutex<Vec<DelayedTask>>,
    state: Arc<AtomicBool>
}

pub enum ScheduleFlow {
    Continue(Duration),
    Break,
}

static mut SCHEDULER: Option<Scheduler> = None;

impl Scheduler {
    pub fn launch() {
        unsafe {
            SCHEDULER = Some(Self {
                tasks: Mutex::new(vec![]),
                state: Arc::new(AtomicBool::new(true))
            });
        }
        let instance = Self::instance_mut();
        thread::spawn(move || {
            while instance.state.load(std::sync::atomic::Ordering::Relaxed) {
                let mut tasks = instance.tasks.lock().unwrap();
                let mut i = 0;
                // Traverse the tasks and push the ready ones
                while i < tasks.len() {
                    if tasks[i].is_ready() {
                        Dispatcher::push(tasks.remove(i).task);
                    } else {
                        i += 1;
                    }
                }
                drop(tasks);
                // Maximum UPS (updates per second) for a active timer is 500
                thread::sleep(Duration::from_millis(2));
            }
        });
    }

    fn instance() -> &'static Scheduler {
        unsafe { SCHEDULER.as_ref().unwrap() }
    }

    fn instance_mut() -> &'static mut Scheduler {
        unsafe { SCHEDULER.as_mut().unwrap() }
    }

    pub fn deploy<F: 'static>(task: F, delay: Duration) where F: FnOnce() + Send {
        let mut tasks = Self::instance().tasks.lock().unwrap();
        tasks.push(DelayedTask::new(Box::new(task), delay));
    }

    pub fn shutdown() {
        Self::instance().state.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    fn deploy_dynamic<F: 'static>(task: F, delay: Duration) where F: Fn() -> ScheduleFlow + Send {
        Self::deploy(move || {
            match task() {
                ScheduleFlow::Continue(delay) => {
                    Self::deploy_dynamic(task, delay);
                }
                ScheduleFlow::Break => {}
            }
        }, delay);
    }

    fn deploy_repeat<F: 'static>(count: usize, interval: Duration, task: F)
        where F: Fn(usize) + Send
    {
        let repeating_task = Box::new(move || {
            task(count);
            if count > 1 {
                Self::deploy_repeat(count - 1, interval, task);
            }
        });
        Self::deploy(repeating_task, interval);
    }
}