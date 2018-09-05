use stats;
use config;

use std::thread;
use std::time;
use std::sync;



pub fn run_reporter(stats: sync::Arc<stats::Stats>, config: sync::Arc<config::Config>, done_flag: sync::Arc<sync::atomic::AtomicBool>){
    let sleep_duration=time::Duration::from_millis(config.report_period);
    drop(config);
    for i in 0..{
        thread::sleep(sleep_duration);
        println!("Report number {}:\n{}\n\n", i, stats);
        if done_flag.load(sync::atomic::Ordering::Relaxed){
            break;
        }
    }

    println!("Reporter rerminated.");
}