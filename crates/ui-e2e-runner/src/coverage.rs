//! Profile artifacts belong to one executed case and its exact image ELFs.
use super::*;

pub(super) fn prepare(artifacts: &Path) -> Result<PathBuf> {
    if !cfg!(storage_ui_coverage) {
        bail!("UI coverage requires an instrumented runner and app image");
    }
    let profiles = artifacts.join("profiles");
    fs::create_dir(&profiles)?;
    #[cfg(storage_ui_coverage)]
    {
        use std::os::unix::ffi::OsStrExt;
        unsafe extern "C" {
            fn __llvm_profile_set_filename(name: *const std::ffi::c_char);
        }
        let filename =
            std::ffi::CString::new(profiles.join("runner-%m-%p.profraw").as_os_str().as_bytes())?;
        // The runtime copies this string; set the filename once before case
        // execution, without changing the multithreaded process environment.
        unsafe {
            __llvm_profile_set_filename(filename.as_ptr());
        }
    }
    Ok(profiles)
}

pub(super) fn checkpoint() -> Result<()> {
    #[cfg(storage_ui_coverage)]
    {
        unsafe extern "C" {
            fn __llvm_profile_write_file() -> std::ffi::c_int;
        }
        if unsafe { __llvm_profile_write_file() } != 0 {
            bail!("runner LLVM profile checkpoint failed");
        }
        Ok(())
    }
    #[cfg(not(storage_ui_coverage))]
    bail!("runner is not instrumented")
}

pub(super) fn save_objects(artifacts: &Path, app: &Path) -> Result<()> {
    let objects = artifacts.join("objects");
    fs::create_dir(&objects)?;
    fs::copy(app, objects.join("application"))?;
    fs::copy(std::env::current_exe()?, objects.join("runner"))?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/coverage_tests.rs"]
mod tests;
