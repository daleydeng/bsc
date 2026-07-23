use std::env;
use std::ffi::OsString;

use anyhow::{Context, Result};
use xshell::{cmd, Shell};

use crate::environment::PreparedEnvironment;

pub struct Tasks<'a> {
    shell: &'a Shell,
    environment: &'a PreparedEnvironment,
    cargo: OsString,
}

impl<'a> Tasks<'a> {
    pub fn new(shell: &'a Shell, environment: &'a PreparedEnvironment) -> Self {
        Self {
            shell,
            environment,
            cargo: env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")),
        }
    }

    pub fn test_z3(&self) -> Result<()> {
        let sh = self.shell;
        let cargo = &self.cargo;
        let jobs = self.environment.jobs.to_string();
        cmd!(
            sh,
            "{cargo} test --locked --package bsc-rust-tests --test scheduler_sat --jobs {jobs} -- --test-threads {jobs}"
        )
        .run()
        .context("scheduler SAT tests failed")?;
        Ok(())
    }

    pub fn test_alignment(&self) -> Result<()> {
        let sh = self.shell;
        let cargo = &self.cargo;
        let jobs = self.environment.jobs.to_string();
        cmd!(
            sh,
            "{cargo} run --locked --package bsc-rust-tests --bin alignment --jobs {jobs}"
        )
        .run()
        .context("upstream alignment check failed")?;
        Ok(())
    }

    pub fn inventory_check(&self) -> Result<()> {
        self.inventory("--check")
    }

    pub fn inventory_update(&self) -> Result<()> {
        self.inventory("--write")
    }

    pub fn test_upstream(&self, arguments: &[String]) -> Result<()> {
        self.test_alignment()?;
        self.inventory_check()?;

        let sh = self.shell;
        let cargo = &self.cargo;
        let jobs = self.environment.jobs.to_string();
        cmd!(
            sh,
            "{cargo} run --locked --package bsc-rust-tests --bin upstream --jobs {jobs} -- {arguments...} --test-threads {jobs}"
        )
        .run()
        .context("migrated upstream tests failed")?;
        Ok(())
    }

    pub fn test_rust(&self) -> Result<()> {
        let sh = self.shell;
        let cargo = &self.cargo;
        let jobs = self.environment.jobs.to_string();
        cmd!(
            sh,
            "{cargo} test --locked --package bsc-rust-tests --lib --test scheduler_sat --jobs {jobs} -- --test-threads {jobs}"
        )
        .run()
        .context("Rust unit or scheduler tests failed")?;
        self.test_upstream(&[])
    }

    pub fn test_cold(&self) -> Result<()> {
        env::set_var("BSC_TEST_CACHE", "0");
        env::set_var("CCACHE_DISABLE", "1");
        if self.environment.ccache_managed_cxx {
            env::set_var("CXX", "c++");
        }
        self.test_rust()
    }

    pub fn ccache_stats(&self) -> Result<()> {
        let sh = self.shell;
        cmd!(sh, "ccache.exe --show-stats")
            .run()
            .context("could not read ccache statistics")?;
        Ok(())
    }

    pub fn ccache_clear(&self) -> Result<()> {
        let sh = self.shell;
        cmd!(sh, "ccache.exe --clear")
            .run()
            .context("could not clear ccache")?;
        Ok(())
    }

    fn inventory(&self, mode: &str) -> Result<()> {
        let sh = self.shell;
        let cargo = &self.cargo;
        let jobs = self.environment.jobs.to_string();
        cmd!(
            sh,
            "{cargo} run --locked --package bsc-rust-tests --bin remaining --jobs {jobs} -- {mode}"
        )
        .run()
        .with_context(|| format!("remaining-tests inventory {mode} failed"))?;
        Ok(())
    }
}
