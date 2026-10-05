#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => (tracing::error!(target: TARGET, $($arg)*));
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => (tracing::warn!(target: TARGET, $($arg)*));
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => (tracing::info!(target: TARGET, $($arg)*));
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => (tracing::debug!(target: TARGET, $($arg)*));
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => (tracing::trace!(target: TARGET, $($arg)*));
}