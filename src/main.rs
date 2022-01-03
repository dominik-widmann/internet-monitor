use chrono::{DateTime, Utc};
use rand::random;
use std::error::Error;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;
use std::{thread, time};

use std::io::prelude::*;

fn google_url_to_ipv4() -> Result<SocketAddr, io::Error> {
    // resolve google
    let mut socket_addrs_iter = "www.google.com:80".to_socket_addrs()?;

    // convert socket addr to ipv4 addr
    socket_addrs_iter.next();
    let socket_addrs = match socket_addrs_iter.next() {
        Some(val) => val,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "No valid IP address found.",
            ))
        }
    };
    Ok(socket_addrs)
}

fn ping_google() -> Result<(), io::Error> {
    // resolve google ip as socket addr. If it doesn't work continue.
    let google_addr = match google_url_to_ipv4() {
        Ok(addr) => addr,
        Err(e) => {
            println!("{:?}: Name resolution failed with: '{}'", Utc::now(), e);
            return Err(e);
        }
    };
    // ping google with a timeout of 1 second
    let timeout = Duration::from_secs(1);
    match ping::ping(
        google_addr.ip(),
        Some(timeout),
        Some(166),
        Some(3),
        Some(5),
        Some(&random()),
    ) {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("{:?}: Ping failed with: '{}'", Utc::now(), e);
            Err(io::Error::new(io::ErrorKind::Other, e))
        }
    }
}

struct InternetMonitor {
    logfile: File,
    is_inet_available: bool,
    last_disconnect_time: DateTime<Utc>,
}

impl InternetMonitor {
    fn new() -> Result<InternetMonitor, io::Error> {
        // create a log file
        let mut logfile = OpenOptions::new()
            .append(true)
            .create(true)
            .open("internet.log")?;
        write!(&mut logfile, "Event,    Time,   Duration\n")?;
        Ok(InternetMonitor {
            logfile,
            is_inet_available: true,
            last_disconnect_time: Utc::now(),
        })
    }

    /// Checks if internet is available, if not writes timestamps to log file
    fn check_connected(&mut self) {
        match ping_google() {
            Ok(_) => {
                if !self.is_inet_available {
                    // Internet has just become available again, log that
                    self.is_inet_available = true;
                    match write!(
                        &mut self.logfile,
                        "{}\n",
                        Utc::now().signed_duration_since(self.last_disconnect_time)
                    ) {
                        Ok(_) => (),
                        Err(e) => println!("Logging error: {}", e),
                    }
                }
            }
            Err(_) => {
                // if this was newly discovered, log the start of the internet outage
                if self.is_inet_available {
                    self.is_inet_available = false;
                    self.last_disconnect_time = Utc::now();
                    match write!(
                        &mut self.logfile,
                        "Internet unavailable,  {}, ",
                        self.last_disconnect_time
                    ) {
                        Ok(_) => (),
                        Err(e) => println!("Logging error: {}", e),
                    }
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // ensure we have elevated privileges
    sudo::escalate_if_needed()?;

    // create an internet monitor
    let mut monitor = InternetMonitor::new().unwrap();

    println!("Start monitoring internet connectivity...");
    // start pinging google
    let sleep_time = time::Duration::from_secs(1);
    loop {
        thread::sleep(sleep_time);
        monitor.check_connected();
    }
}
