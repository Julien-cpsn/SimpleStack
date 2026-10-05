use crate::{DATABASE_URL, GNS3_PROJECT_PREFIX, GNS3_SERVER_PASSWORD, GNS3_SERVER_URL, GNS3_SERVER_USERNAME, GNS3_TEMPLATE_PREFIX};
use dotenv::dotenv;
use std::env;

pub fn harvest_env_variables() {
    dotenv().ok();

    for (key, value) in env::vars() {
        match key.as_str() {
            "DATABASE_URL" => { DATABASE_URL.get_or_init(|| value); },
            "GNS3_SERVER_URL" => { GNS3_SERVER_URL.get_or_init(|| value); },
            "GNS3_SERVER_USERNAME" => { GNS3_SERVER_USERNAME.get_or_init(|| value); },
            "GNS3_SERVER_PASSWORD" => { GNS3_SERVER_PASSWORD.get_or_init(|| value); }
            "GNS3_PROJECT_PREFIX" => { GNS3_PROJECT_PREFIX.get_or_init(|| value); }
            "GNS3_TEMPLATE_PREFIX" => { GNS3_TEMPLATE_PREFIX.get_or_init(|| value); }
            _ => {}
        }
    }

    if DATABASE_URL.get().is_none() {
        panic!("Environment variable DATABASE_URL not set");
    }

    if GNS3_SERVER_URL.get().is_none() {
        panic!("Environment variable GNS3_SERVER_URL not set");
    }

    if GNS3_SERVER_USERNAME.get().is_none() {
        panic!("Environment variable GNS3_SERVER_USERNAME not set");
    }

    if GNS3_SERVER_PASSWORD.get().is_none() {
        panic!("Environment variable GNS3_SERVER_PASSWORD not set");
    }

    if GNS3_PROJECT_PREFIX.get().is_none() {
        panic!("Environment variable GNS3_PROJECT_PREFIX not set");
    }

    if GNS3_TEMPLATE_PREFIX.get().is_none() {
        panic!("Environment variable GNS3_TEMPLATE_PREFIX not set");
    }
}