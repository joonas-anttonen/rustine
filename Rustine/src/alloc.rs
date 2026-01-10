use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct TrackingAllocator;

static CURRENT_BYTES: AtomicUsize = AtomicUsize::new(0);
static TOTAL_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        TOTAL_BYTES.fetch_add(size, Ordering::Relaxed);
        let curr = CURRENT_BYTES.fetch_add(size, Ordering::Relaxed) + size;

        let mut peak = PEAK_BYTES.load(Ordering::Relaxed);
        while curr > peak
            && PEAK_BYTES
                .compare_exchange_weak(peak, curr, Ordering::Relaxed, Ordering::Relaxed)
                .is_err()
        {
            peak = PEAK_BYTES.load(Ordering::Relaxed);
        }

        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        let size = layout.size();
        CURRENT_BYTES.fetch_sub(size, Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_size = layout.size();
        if new_size >= old_size {
            let add = new_size - old_size;
            let curr = CURRENT_BYTES.fetch_add(add, Ordering::Relaxed) + add;
            let mut peak = PEAK_BYTES.load(Ordering::Relaxed);
            while curr > peak
                && PEAK_BYTES
                    .compare_exchange_weak(peak, curr, Ordering::Relaxed, Ordering::Relaxed)
                    .is_err()
            {
                peak = PEAK_BYTES.load(Ordering::Relaxed);
            }
        } else {
            let sub = old_size - new_size;
            CURRENT_BYTES.fetch_sub(sub, Ordering::Relaxed);
        }

        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: TrackingAllocator = TrackingAllocator;

pub fn current_bytes() -> usize {
    CURRENT_BYTES.load(Ordering::Relaxed)
}

pub fn peak_bytes() -> usize {
    PEAK_BYTES.load(Ordering::Relaxed)
}

pub fn counts() -> (usize, usize) {
    (
        ALLOC_COUNT.load(Ordering::Relaxed),
        DEALLOC_COUNT.load(Ordering::Relaxed),
    )
}

#[cfg(target_os = "linux")]
pub fn rss_bytes() -> Option<usize> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            // Format: "VmRSS:\t   12345 kB"
            let trimmed = rest.trim();
            let kb_str = trimmed
                .strip_suffix("kB")
                .map(|s| s.trim())
                .unwrap_or(trimmed);
            if let Ok(kb) = kb_str.parse::<usize>() {
                return Some(kb * 1024);
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
pub fn rss_bytes() -> Option<usize> {
    None
}
