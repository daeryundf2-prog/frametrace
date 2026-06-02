use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const SAMPLE_INTERVAL: Duration = Duration::from_millis(100);
pub const CPU_TARGET_PERCENT: f64 = 95.0;

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub max_rss_bytes: Option<u64>,
    pub max_rss_target_bytes: u64,
    pub average_cpu_percent: Option<f64>,
    pub cpu_target_percent: f64,
    pub cpu_target_enforced: bool,
    pub sample_count: usize,
}

impl ResourceUsage {
    pub fn metrics_available(&self) -> bool {
        self.max_rss_bytes.is_some() && self.average_cpu_percent.is_some()
    }

    pub fn rss_passed(&self) -> bool {
        matches!(self.max_rss_bytes, Some(bytes) if bytes <= self.max_rss_target_bytes)
    }

    pub fn cpu_passed(&self) -> bool {
        match (self.cpu_target_enforced, self.average_cpu_percent) {
            (true, Some(percent)) => percent <= self.cpu_target_percent,
            (true, None) => false,
            (false, Some(_)) => true,
            (false, None) => false,
        }
    }
}

pub struct ResourceMonitor {
    pid: u32,
    started: Instant,
    start_cpu_ms: Option<u64>,
    stop: Arc<AtomicBool>,
    peak_rss_bytes: Arc<AtomicU64>,
    sample_count: Arc<AtomicUsize>,
    handle: Option<JoinHandle<()>>,
}

impl ResourceMonitor {
    pub fn start() -> Self {
        let pid = std::process::id();
        let initial_rss = current_rss_bytes(pid).unwrap_or(0);
        let stop = Arc::new(AtomicBool::new(false));
        let peak_rss_bytes = Arc::new(AtomicU64::new(initial_rss));
        let sample_count = Arc::new(AtomicUsize::new(usize::from(initial_rss > 0)));
        let handle = spawn_rss_sampler(pid, Arc::clone(&stop), &peak_rss_bytes, &sample_count);

        Self {
            pid,
            started: Instant::now(),
            start_cpu_ms: current_cpu_ms(pid),
            stop,
            peak_rss_bytes,
            sample_count,
            handle: Some(handle),
        }
    }

    pub fn finish(mut self, max_rss_target_bytes: u64, cpu_target_enforced: bool) -> ResourceUsage {
        self.stop_sampler();
        let elapsed_ms = self.started.elapsed().as_millis();
        let average_cpu_percent =
            average_cpu_percent(self.start_cpu_ms, current_cpu_ms(self.pid), elapsed_ms);
        let max_rss = match self.peak_rss_bytes.load(Ordering::Relaxed) {
            0 => None,
            bytes => Some(bytes),
        };

        ResourceUsage {
            max_rss_bytes: max_rss,
            max_rss_target_bytes,
            average_cpu_percent,
            cpu_target_percent: CPU_TARGET_PERCENT,
            cpu_target_enforced,
            sample_count: self.sample_count.load(Ordering::Relaxed),
        }
    }

    fn stop_sampler(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for ResourceMonitor {
    fn drop(&mut self) {
        self.stop_sampler();
    }
}

fn spawn_rss_sampler(
    pid: u32,
    stop: Arc<AtomicBool>,
    peak_rss_bytes: &Arc<AtomicU64>,
    sample_count: &Arc<AtomicUsize>,
) -> JoinHandle<()> {
    let peak_rss_bytes = Arc::clone(peak_rss_bytes);
    let sample_count = Arc::clone(sample_count);
    thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            if let Some(bytes) = current_rss_bytes(pid) {
                record_peak(&peak_rss_bytes, bytes);
                sample_count.fetch_add(1, Ordering::Relaxed);
            }
            thread::sleep(SAMPLE_INTERVAL);
        }
    })
}

fn record_peak(peak_rss_bytes: &AtomicU64, bytes: u64) {
    let mut current = peak_rss_bytes.load(Ordering::Relaxed);
    while bytes > current {
        match peak_rss_bytes.compare_exchange(current, bytes, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

fn average_cpu_percent(
    start_cpu_ms: Option<u64>,
    end_cpu_ms: Option<u64>,
    elapsed_ms: u128,
) -> Option<f64> {
    let start = start_cpu_ms?;
    let end = end_cpu_ms?;
    if elapsed_ms == 0 || end < start {
        return Some(0.0);
    }
    Some((end - start) as f64 * 100.0 / elapsed_ms as f64)
}

#[cfg(unix)]
fn current_rss_bytes(pid: u32) -> Option<u64> {
    let text = command_stdout("ps", &["-o", "rss=", "-p", &pid.to_string()])?;
    text.trim().parse::<u64>().ok().map(|kb| kb * 1024)
}

#[cfg(windows)]
fn current_rss_bytes(pid: u32) -> Option<u64> {
    let script = format!("[int64]((Get-Process -Id {pid}).WorkingSet64)");
    command_stdout("powershell", &["-NoProfile", "-Command", &script])?
        .trim()
        .parse::<u64>()
        .ok()
}

#[cfg(not(any(unix, windows)))]
fn current_rss_bytes(_pid: u32) -> Option<u64> {
    None
}

#[cfg(unix)]
fn current_cpu_ms(pid: u32) -> Option<u64> {
    let text = command_stdout("ps", &["-o", "time=", "-p", &pid.to_string()])?;
    parse_ps_time_ms(text.trim())
}

#[cfg(windows)]
fn current_cpu_ms(pid: u32) -> Option<u64> {
    let script = format!("[int64]((Get-Process -Id {pid}).TotalProcessorTime.TotalMilliseconds)");
    command_stdout("powershell", &["-NoProfile", "-Command", &script])?
        .trim()
        .parse::<u64>()
        .ok()
}

#[cfg(not(any(unix, windows)))]
fn current_cpu_ms(_pid: u32) -> Option<u64> {
    None
}

fn command_stdout(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

#[cfg(unix)]
fn parse_ps_time_ms(input: &str) -> Option<u64> {
    let (days, time) = match input.split_once('-') {
        Some((days, time)) => (days.trim().parse::<u64>().ok()?, time),
        None => (0, input),
    };
    let parts = time.split(':').collect::<Vec<_>>();
    let (hours, minutes, seconds) = match parts.as_slice() {
        [minutes, seconds] => (0, parse_u64(minutes)?, parse_seconds_ms(seconds)?),
        [hours, minutes, seconds] => (
            parse_u64(hours)?,
            parse_u64(minutes)?,
            parse_seconds_ms(seconds)?,
        ),
        [seconds] => (0, 0, parse_seconds_ms(seconds)?),
        _ => return None,
    };
    Some(days * 86_400_000 + hours * 3_600_000 + minutes * 60_000 + seconds)
}

#[cfg(unix)]
fn parse_u64(input: &str) -> Option<u64> {
    input.trim().parse::<u64>().ok()
}

#[cfg(unix)]
fn parse_seconds_ms(input: &str) -> Option<u64> {
    let trimmed = input.trim();
    let (whole, fraction) = trimmed.split_once('.').unwrap_or((trimmed, ""));
    let whole_ms = whole.parse::<u64>().ok()? * 1_000;
    let mut fraction_text = fraction.chars().take(3).collect::<String>();
    while fraction_text.len() < 3 {
        fraction_text.push('0');
    }
    Some(whole_ms + fraction_text.parse::<u64>().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::parse_ps_time_ms;
    use super::{ResourceUsage, average_cpu_percent};

    #[test]
    fn resource_usage_requires_available_metrics() {
        let usage = ResourceUsage {
            max_rss_bytes: Some(100),
            max_rss_target_bytes: 200,
            average_cpu_percent: Some(12.5),
            cpu_target_percent: 95.0,
            cpu_target_enforced: true,
            sample_count: 2,
        };

        assert!(usage.metrics_available());
        assert!(usage.rss_passed());
        assert!(usage.cpu_passed());
    }

    #[test]
    fn average_cpu_percent_uses_elapsed_wall_time() {
        assert_eq!(average_cpu_percent(Some(100), Some(250), 300), Some(50.0));
    }

    #[cfg(unix)]
    #[test]
    fn parses_ps_cpu_time_variants() {
        assert_eq!(parse_ps_time_ms("0:00.01"), Some(10));
        assert_eq!(parse_ps_time_ms("01:02:03"), Some(3_723_000));
        assert_eq!(parse_ps_time_ms("2-01:02:03"), Some(176_523_000));
    }
}
