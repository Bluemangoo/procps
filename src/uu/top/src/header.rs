use crate::picker::{sysinfo, systemstat};
use bytesize::ByteSize;
use clap::ArgMatches;
use systemstat::Platform;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
use {
    std::sync::{Mutex, OnceLock},
    systemstat::{CPULoad, DelayedMeasurement},
};

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
static LOAD_AVERAGE: OnceLock<Mutex<DelayedMeasurement<CPULoad>>> = OnceLock::new();

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(crate) fn cpu_load() -> CPULoad {
    match LOAD_AVERAGE.get() {
        None => {
            LOAD_AVERAGE.get_or_init(|| {
                Mutex::new(systemstat().read().unwrap().cpu_load_aggregate().unwrap())
            });
            systemstat()
                .read()
                .unwrap()
                .cpu_load_aggregate()
                .unwrap()
                .done()
                .unwrap()
        }
        Some(v) => {
            let mut v = v.lock().unwrap();
            let load = v.done().unwrap();
            *v = systemstat().read().unwrap().cpu_load_aggregate().unwrap();
            load
        }
    }
}

pub(crate) fn header(arg: &ArgMatches) -> String {
    format!(
        "top - {time} {uptime}, {user}, {load_average}\n\
        {task}\n\
        {cpu}\n\
        {memory}",
        time = chrono::Local::now().format("%H:%M:%S"),
        uptime = uptime(),
        user = user(),
        load_average = load_average(),
        task = task(),
        cpu = cpu(),
        memory = memory(arg),
    )
}

fn todo() -> String {
    "TODO".into()
}

fn format_memory(memory_b: u64, unit: u64) -> f64 {
    ByteSize::b(memory_b).0 as f64 / unit as f64
}

fn uptime() -> String {
    let binding = systemstat().read().unwrap();

    let up_seconds = binding.uptime().unwrap().as_secs();
    let up_minutes = (up_seconds % (60 * 60)) / 60;
    let up_hours = (up_seconds % (24 * 60 * 60)) / (60 * 60);
    let up_days = up_seconds / (24 * 60 * 60);

    let mut res = String::from("up ");

    if up_days > 0 {
        res.push_str(&format!(
            "{} day{}, ",
            up_days,
            if up_days > 1 { "s" } else { "" }
        ));
    }
    if up_hours > 0 {
        res.push_str(&format!("{}:{:0>2}", up_hours, up_minutes));
    } else {
        res.push_str(&format!("{} min", up_minutes));
    }

    res
}

//TODO: Implement active user count
fn user() -> String {
    todo()
}

#[cfg(not(target_os = "windows"))]
fn load_average() -> String {
    let binding = systemstat().read().unwrap();

    let load_average = binding.load_average().unwrap();
    format!(
        "load average: {:.2}, {:.2}, {:.2}",
        load_average.one, load_average.five, load_average.fifteen
    )
}

#[cfg(target_os = "windows")]
fn load_average() -> String {
    todo()
}

fn task() -> String {
    let binding = sysinfo().read().unwrap();

    let process = binding.processes();
    let mut running_process = 0;
    let mut sleeping_process = 0;
    let mut stopped_process = 0;
    let mut zombie_process = 0;

    for (_, process) in process.iter() {
        match process.status() {
            sysinfo::ProcessStatus::Run => running_process += 1,
            sysinfo::ProcessStatus::Sleep => sleeping_process += 1,
            sysinfo::ProcessStatus::Stop => stopped_process += 1,
            sysinfo::ProcessStatus::Zombie => zombie_process += 1,
            _ => {}
        };
    }

    format!(
        "Tasks: {} total, {} running, {} sleeping, {} stopped, {} zombie",
        process.len(),
        running_process,
        sleeping_process,
        stopped_process,
        zombie_process,
    )
}

#[cfg(target_os = "linux")]
fn cpu() -> String {
    let file = std::fs::File::open(std::path::Path::new("/proc/stat")).unwrap();
    let content = std::io::read_to_string(file).unwrap();
    let load = content
        .lines()
        .next()
        .unwrap()
        .strip_prefix("cpu")
        .unwrap()
        .split(' ')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let user = load[0].parse::<f64>().unwrap();
    let nice = load[1].parse::<f64>().unwrap();
    let system = load[2].parse::<f64>().unwrap();
    let idle = load[3].parse::<f64>().unwrap_or_default(); // since 2.5.41
    let io_wait = load[4].parse::<f64>().unwrap_or_default(); // since 2.5.41
    let hardware_interrupt = load[5].parse::<f64>().unwrap_or_default(); // since 2.6.0
    let software_interrupt = load[6].parse::<f64>().unwrap_or_default(); // since 2.6.0
    let steal_time = load[7].parse::<f64>().unwrap_or_default(); // since 2.6.11
                                                                 // GNU do not show guest and guest_nice
    let guest = load[8].parse::<f64>().unwrap_or_default(); // since 2.6.24
    let guest_nice = load[9].parse::<f64>().unwrap_or_default(); // since 2.6.33
    let total = user
        + nice
        + system
        + idle
        + io_wait
        + hardware_interrupt
        + software_interrupt
        + steal_time
        + guest
        + guest_nice;

    format!(
        "%Cpu(s):  {:.1} us, {:.1} sy, {:.1} ni, {:.1} id, {:.1} wa, {:.1} hi, {:.1} si, {:.1} st",
        user / total * 100.0,
        system / total * 100.0,
        nice / total * 100.0,
        idle / total * 100.0,
        io_wait / total * 100.0,
        hardware_interrupt / total * 100.0,
        software_interrupt / total * 100.0,
        steal_time / total * 100.0,
    )
}

//TODO: Implement io wait, hardware interrupt, software interrupt and steal time
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn cpu() -> String {
    let cpu = cpu_load();
    format!(
        "%Cpu(s):  {:.1} us,  {:.1} sy,  {:.1} ni, {:.1} id",
        cpu.user * 100.0,
        cpu.system * 100.0,
        cpu.nice * 100.0,
        cpu.idle * 100.0
    )
}

//TODO: Implement for macos
#[cfg(target_os = "macos")]
fn cpu() -> String {
    todo()
}

fn memory(arg: &ArgMatches) -> String {
    let binding = sysinfo().read().unwrap();
    let (unit, unit_name) = match arg.get_one::<String>("scale-summary-mem") {
        Some(scale) => match scale.as_str() {
            "k" => (bytesize::KIB, "KiB"),
            "m" => (bytesize::MIB, "MiB"),
            "g" => (bytesize::GIB, "GiB"),
            "t" => (bytesize::TIB, "TiB"),
            "p" => (bytesize::PIB, "PiB"),
            "e" => (1_152_921_504_606_846_976, "EiB"),
            _ => (bytesize::MIB, "MiB"),
        },
        None => {
            (bytesize::MIB, "MiB")
        }
    };

    format!(
        "{unit_name} Mem : {:8.1} total, {:8.1} free, {:8.1} used, {:8.1} buff/cache\n\
        {unit_name} Swap: {:8.1} total, {:8.1} free, {:8.1} used, {:8.1} avail Mem",
        format_memory(binding.total_memory(), unit),
        format_memory(binding.free_memory(), unit),
        format_memory(binding.used_memory(), unit),
        format_memory(binding.available_memory() - binding.free_memory(), unit),
        format_memory(binding.total_swap(), unit),
        format_memory(binding.free_swap(), unit),
        format_memory(binding.used_swap(), unit),
        format_memory(binding.available_memory(), unit),
        unit_name = unit_name
    )
}
