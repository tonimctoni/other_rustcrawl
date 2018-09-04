use std::sync;
use std::fmt;


pub struct Stats{
    pub urlreservoir_size: sync::atomic::AtomicUsize,
    pub urlreservoir_add_calls: sync::atomic::AtomicUsize,
    pub urlreservoir_add_addedurls: sync::atomic::AtomicUsize,

    pub urlreservoir_getuu_calls: sync::atomic::AtomicUsize,
    pub urlreservoir_getuu_locked: sync::atomic::AtomicUsize,
    pub urlreservoir_getuu_emptyqueue: sync::atomic::AtomicUsize,
    pub urlreservoir_getuu_illformat: sync::atomic::AtomicUsize,
    pub urlreservoir_getuu_success: sync::atomic::AtomicUsize,

    pub client_work_sent: sync::atomic::AtomicUsize,
    pub client_hyper_error: sync::atomic::AtomicUsize,
    pub client_timeout: sync::atomic::AtomicUsize,
    pub client_timeout_error: sync::atomic::AtomicUsize,
    pub client_too_large_error: sync::atomic::AtomicUsize,
    pub client_unwanted_file: sync::atomic::AtomicUsize,

    pub worker_got_html: sync::atomic::AtomicUsize,
    pub worker_got_file: sync::atomic::AtomicUsize,
    pub worker_got_repeated_file: sync::atomic::AtomicUsize,
    pub worker_write_file_error: sync::atomic::AtomicUsize,
}

impl Stats {
    pub fn new() -> sync::Arc<Stats>{
        sync::Arc::new(Stats{
            urlreservoir_size: sync::atomic::AtomicUsize::new(0),
            urlreservoir_add_calls: sync::atomic::AtomicUsize::new(0),
            urlreservoir_add_addedurls: sync::atomic::AtomicUsize::new(0),

            urlreservoir_getuu_calls: sync::atomic::AtomicUsize::new(0),
            urlreservoir_getuu_locked: sync::atomic::AtomicUsize::new(0),
            urlreservoir_getuu_emptyqueue: sync::atomic::AtomicUsize::new(0),
            urlreservoir_getuu_illformat: sync::atomic::AtomicUsize::new(0),
            urlreservoir_getuu_success: sync::atomic::AtomicUsize::new(0),

            client_work_sent: sync::atomic::AtomicUsize::new(0),
            client_hyper_error: sync::atomic::AtomicUsize::new(0),
            client_timeout: sync::atomic::AtomicUsize::new(0),
            client_timeout_error: sync::atomic::AtomicUsize::new(0),
            client_too_large_error: sync::atomic::AtomicUsize::new(0),
            client_unwanted_file: sync::atomic::AtomicUsize::new(0),

            worker_got_html: sync::atomic::AtomicUsize::new(0),
            worker_got_file: sync::atomic::AtomicUsize::new(0),
            worker_got_repeated_file: sync::atomic::AtomicUsize::new(0),
            worker_write_file_error: sync::atomic::AtomicUsize::new(0),
        })
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
        .and_then(|_|write!(f, "UrlReservoirAdd\n"))
        .and_then(|_|write!(f, "----reservoir size: {}\n", self.urlreservoir_size.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "----add_urls calls: {}\n", self.urlreservoir_add_calls.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "----urls added: {}\n", self.urlreservoir_add_addedurls.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "UrlReservoirGetuu\n"))
        .and_then(|_|write!(f, "----calls: {}\n", self.urlreservoir_getuu_calls.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "----error locked: {}\n", self.urlreservoir_getuu_locked.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "----error emptyqueue: {}\n", self.urlreservoir_getuu_emptyqueue.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "----error illformat: {}\n", self.urlreservoir_getuu_illformat.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "----success: {}\n", self.urlreservoir_getuu_success.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "Client\n"))
        .and_then(|_|write!(f, "----work sent: {}\n", self.client_work_sent.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "----hyper error: {}\n", self.client_hyper_error.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "----timeout: {}\n", self.client_timeout.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "----timeout error: {}\n", self.client_timeout_error.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "----too large error: {}\n", self.client_too_large_error.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "----unwanted file: {}\n", self.client_unwanted_file.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "Worker\n"))
        .and_then(|_|write!(f, "----got total: {}\n", self.worker_got_html.load(sync::atomic::Ordering::Relaxed)+self.worker_got_file.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "----got html: {}\n", self.worker_got_html.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "----got file: {}\n", self.worker_got_file.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "----got repeated file: {}\n", self.worker_got_repeated_file.load(sync::atomic::Ordering::Relaxed)))
        .and_then(|_|write!(f, "----got write file error: {}\n", self.worker_write_file_error.load(sync::atomic::Ordering::Relaxed)))
    }
}
