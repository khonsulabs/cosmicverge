// These values are provided by /build.rs

#[macro_export]
macro_rules! current_git_revision {
    () => {
        std::env!("GIT_REF")
    };
}

#[macro_export]
macro_rules! current_short_git_revision {
    () => {
        std::env!("GIT_SHORT_REF")
    };
}

#[macro_export]
macro_rules! current_git_timestamp {
    () => {
        std::env!("GIT_TIMESTAMP").parse().unwrap()
    };
}
