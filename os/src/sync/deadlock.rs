use core::cell::RefMut;

use alloc::vec;
use alloc::vec::Vec;

use super::UPSafeCell;

/// A wrapper struct for deadlock detection that provides thread-safe access to the detector.
#[allow(unused)]
pub struct DeadLockDetector {
    inner: UPSafeCell<DeadlockDetectorInner>,
}

#[allow(unused)]
impl DeadLockDetector {
    /// create a deadlock detector impl with banker
    pub fn new() -> Self {
        Self {
            inner: unsafe { UPSafeCell::new(DeadlockDetectorInner::new()) },
        }
    }

    /// Get the mutable reference of the inner TCB
    pub fn inner_exclusive_access(&self) -> RefMut<'_, DeadlockDetectorInner> {
        self.inner.exclusive_access()
    }
}

/// A struct for detecting potential deadlocks in the system.
#[derive(Clone, Debug)]
pub struct DeadlockDetectorInner {
    /// is deadlock detector enabled
    pub enabled: bool,

    /// available matrix
    pub available: Vec<usize>,

    /// allocation matrix
    pub allocation: Vec<Vec<usize>>,

    /// request matrix
    pub request: Vec<Vec<usize>>,

    /// thread count, increase only
    pub thread_count: usize,

    /// resource count, increase only
    pub resource_count: usize,
}

#[allow(unused)]
impl DeadlockDetectorInner {
    /// Creates a new deadlock detector with default settings.
    ///
    /// Returns an empty detector with all matrices initialized to empty vectors and disabled by default.
    pub fn new() -> Self {
        Self {
            enabled: false,
            available: vec![],
            allocation: vec![vec![]],
            request: vec![vec![]],
            // process thread count, default is 1 (main thread)
            thread_count: 1,
            resource_count: 0,
        }
    }

    /// request resource
    pub fn request_resource(&mut self, tid: usize, res_id: usize, res_cnt: usize) -> bool {
        if !self.enabled {
            self.allocation[tid][res_id] += res_cnt;
            self.available[res_id] -= res_cnt;
            return true;
        }

        self.request[tid][res_id] += res_cnt;

        let safe = {
            // safety stimulation
            let mut finished = vec![false; self.thread_count];
            let mut work = self.available.clone();

            loop {
                let brk = (0..self.thread_count)
                    .into_iter()
                    .map(|tid| {
                        if !finished[tid]
                            && (0..self.resource_count)
                                .into_iter()
                                .map(|rid| work[rid] >= self.request[tid][rid])
                                .all(|x| x)
                        {
                            // thread exit
                            finished[tid] = true;
                            // restore allocation matrix
                            (0..self.resource_count).into_iter().for_each(|rid| {
                                work[rid] += self.allocation[tid][rid];
                            });

                            true
                        } else {
                            false
                        }
                    })
                    .all(|x| x == false);
                if brk {
                    break;
                }
            }
            finished.iter().all(|x| *x)
        };

        if safe {
            // if current available matrix is enough for request matrix
            // we can allocate the resource
            // else, we need to save the request matrix
            // request[tid][res_id] = k, means Thread(tid) is waiting(sleep/block) for k resources of res_id
            if self.available[res_id] >= res_cnt {
                self.request[tid][res_id] -= res_cnt;
                self.allocation[tid][res_id] += res_cnt;
                self.available[res_id] -= res_cnt;
            }
            true
        } else {
            // not safe, restore the data
            self.request[tid][res_id] -= res_cnt;
            false
        }
    }

    /// release resource
    pub fn release_resource(&mut self, tid: usize, res_id: usize, res_cnt: usize) -> bool {
        let alloc_cnt = self.allocation[tid][res_id];
        if res_cnt > alloc_cnt {
            return false;
        }

        self.allocation[tid][res_id] -= res_cnt;
        self.available[res_id] += res_cnt;
        true
    }

    /// A resource (mutex / semaphore) created by process
    /// assume the resource will never be deleted.
    pub fn create_resource(&mut self, res_id: usize, res_cnt: usize) {
        // if current length of resource array less then res_id
        // e.g. available.length or allocation[tid].length
        if res_id >= self.resource_count {
            while res_id >= self.resource_count {
                // since we don't know the new element's index is res_id
                // so we set value with 0 first, set available[res_id] = res_cnt etc. later
                self.available.push(0);

                // update matrix
                self.allocation.iter_mut().for_each(|x| x.push(0));
                self.request.iter_mut().for_each(|x| x.push(0));

                self.resource_count += 1;
            }

            self.available[res_id] = res_cnt;
        } else {
            self.available[res_id] = res_cnt;

            // update matrix
            self.allocation.iter_mut().for_each(|x| x[res_id] = 0);
            self.request.iter_mut().for_each(|x| x[res_id] = 0);
        }
    }

    fn clear_matrix_for_tid(&mut self, tid: usize) {
        for rid in 0..self.resource_count {
            self.available[rid] += self.allocation[tid][rid];
        }
        self.allocation[tid].iter_mut().for_each(|v| *v = 0);
        self.request[tid].iter_mut().for_each(|v| *v = 0);
    }

    /// A thread created by process
    pub fn thread_join(&mut self, tid: usize) {
        if tid >= self.thread_count {
            while tid >= self.thread_count {
                self.allocation.push(vec![0; self.resource_count]);
                self.request.push(vec![0; self.resource_count]);
                self.thread_count += 1;
            }
        } else {
            self.clear_matrix_for_tid(tid);
        }
    }

    /// A thread exited
    pub fn thread_exit(&mut self, tid: usize) {
        // restore available
        self.clear_matrix_for_tid(tid);
    }
}
