use std::ptr::{read_volatile, write_volatile};
use std::sync::atomic::{fence, Ordering};
use std::thread;

const NUM_THREADS: usize = 4;
const NUM_LOOP: usize = 100000;

struct BakeryLock {
    entering: [bool; NUM_THREADS],
    tickets: [Option<u64>; NUM_THREADS],
}

impl BakeryLock {
    fn lock(&mut self, idx: usize) -> LockGuard {
        fence(Ordering::SeqCst);
        unsafe { write_volatile(&mut self.entering[idx], true) };
        fence(Ordering::SeqCst);

        let mut max = 0;
        for i in 0..NUM_THREADS {
            if let Some(t) = unsafe { read_volatile(&self.tickets[i]) } {
                max = max.max(t);
            }
        }
        let ticket = max + 1;
        unsafe { write_volatile(&mut self.tickets[idx], Some(ticket)) }

        fence(Ordering::SeqCst);
        unsafe { write_volatile(&mut self.entering[idx], false) };
        fence(Ordering::SeqCst);

        for i in 0..NUM_THREADS {
            if i == idx {
                continue;
            }

            fence(Ordering::SeqCst);
            while unsafe { read_volatile(&self.entering[i]) } {}
            fence(Ordering::SeqCst);

            loop {
                match unsafe { read_volatile(&self.tickets[i]) } {
                    Some(t) => {
                        if ticket < t || (ticket == t && idx < i) {
                            break;
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
        }

        fence(Ordering::SeqCst);
        LockGuard { idx }
    }
}

struct LockGuard {
    idx: usize,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        fence(Ordering::SeqCst);
        unsafe { write_volatile(&mut LOCK.tickets[self.idx], None) };
        fence(Ordering::SeqCst);
    }
}

static mut LOCK: BakeryLock = BakeryLock {
    entering: [false; NUM_THREADS],
    tickets: [None; NUM_THREADS],
};

static mut COUNT: u64 = 0;

fn main() {
    let mut v = Vec::new();
    for i in 0..NUM_THREADS {
        let th = thread::spawn(move || {
            for _ in 0..NUM_LOOP {
                let _lock = unsafe { LOCK.lock(i) };
                unsafe {
                    let c = read_volatile(&COUNT);
                    write_volatile(&mut COUNT, c + 1);
                }
            }
        });
        v.push(th);
    }

    for th in v {
        th.join().unwrap();
    }

    println!(
        "COUNT = {} (expected = {})",
        unsafe { COUNT },
        NUM_LOOP * NUM_THREADS
    );
}
