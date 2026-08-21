use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    ptr, slice, str,
    sync::{
        mpsc::{self, Receiver, SyncSender},
        OnceLock,
    },
    thread,
};

use z3::{SatResult, Solver};

/// Result returned by `bsc_z3_check_smtlib2`.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BscZ3Result {
    Sat = 0,
    Unsat = 1,
    Unknown = 2,
    Error = 3,
}

/// Check one isolated SMT-LIB2 query.
///
/// The input must contain declarations and assertions, but not `check-sat`.
/// A fresh Z3 solver is created for every call, preserving the compiler's
/// existing query-isolation semantics without starting an external process.
/// On failure, `error_output` receives a NUL-terminated UTF-8 diagnostic.
///
/// # Safety
///
/// `input` must point to `input_len` readable bytes for the duration of this
/// call. When `error_capacity` is nonzero, `error_output` must point to that
/// many writable bytes. The input bytes must be valid UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bsc_z3_check_smtlib2(
    input: *const u8,
    input_len: usize,
    error_output: *mut u8,
    error_capacity: usize,
) -> BscZ3Result {
    // SAFETY: The caller owns error_output and guarantees its capacity.
    unsafe { clear_error_output(error_output, error_capacity) };
    match catch_unwind(AssertUnwindSafe(|| check_raw_input(input, input_len))) {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => {
            // SAFETY: The caller owns error_output and guarantees its capacity.
            unsafe { write_error(error_output, error_capacity, &error) };
            BscZ3Result::Error
        }
        Err(payload) => {
            let error = format!("Rust Z3 bridge panicked: {}", panic_message(payload));
            // SAFETY: The caller owns error_output and guarantees its capacity.
            unsafe { write_error(error_output, error_capacity, &error) };
            BscZ3Result::Error
        }
    }
}

struct SolverRequest {
    script: String,
    response: SyncSender<Result<BscZ3Result, String>>,
}

static SOLVER_WORKER: OnceLock<Result<SyncSender<SolverRequest>, String>> = OnceLock::new();

/// Copy the linked Z3 library's full version as NUL-terminated UTF-8.
///
/// The return value is the complete byte length excluding the terminator. A
/// null output or zero capacity can be used to query the required size.
///
/// # Safety
///
/// When `output_capacity` is nonzero, `output` must point to that many writable
/// bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bsc_z3_version(output: *mut u8, output_capacity: usize) -> usize {
    match catch_unwind(z3::full_version) {
        Ok(version) => {
            // SAFETY: The caller owns output and guarantees its capacity.
            unsafe { write_output(output, output_capacity, version) };
            version.len()
        }
        Err(_) => {
            // SAFETY: The caller owns output and guarantees its capacity.
            unsafe { clear_error_output(output, output_capacity) };
            0
        }
    }
}

fn check_raw_input(input: *const u8, input_len: usize) -> Result<BscZ3Result, String> {
    if input.is_null() {
        return Err("SMT-LIB2 input pointer is null".to_owned());
    }
    // SAFETY: Validity for input_len readable bytes is the caller's contract.
    let input = unsafe { slice::from_raw_parts(input, input_len) };
    let script = str::from_utf8(input)
        .map_err(|error| format!("SMT-LIB2 input is not valid UTF-8: {error}"))?;
    check_on_worker(script.to_owned())
}

fn check_on_worker(script: String) -> Result<BscZ3Result, String> {
    let sender = match SOLVER_WORKER.get_or_init(start_solver_worker) {
        Ok(sender) => sender,
        Err(error) => return Err(error.clone()),
    };
    let (response, receiver) = mpsc::sync_channel(0);
    sender
        .send(SolverRequest { script, response })
        .map_err(|_| "Rust Z3 worker stopped before accepting the query".to_owned())?;
    receiver
        .recv()
        .map_err(|_| "Rust Z3 worker stopped before returning the query result".to_owned())?
}

fn start_solver_worker() -> Result<SyncSender<SolverRequest>, String> {
    let (sender, receiver) = mpsc::sync_channel(0);
    thread::Builder::new()
        .name("bsc-z3-solver".to_owned())
        .spawn(move || solver_worker(receiver))
        .map_err(|error| format!("could not start Rust Z3 worker: {error}"))?;
    Ok(sender)
}

fn solver_worker(receiver: Receiver<SolverRequest>) {
    for request in receiver {
        let result = catch_unwind(AssertUnwindSafe(|| check_script(&request.script)))
            .map_err(|payload| format!("Rust Z3 worker panicked: {}", panic_message(payload)));
        let _ = request.response.send(result);
    }
}

fn check_script(script: &str) -> BscZ3Result {
    let solver = Solver::new();
    solver.from_string(script);
    match solver.check() {
        SatResult::Sat => BscZ3Result::Sat,
        SatResult::Unsat => BscZ3Result::Unsat,
        SatResult::Unknown => BscZ3Result::Unknown,
    }
}

unsafe fn clear_error_output(output: *mut u8, capacity: usize) {
    if !output.is_null() && capacity > 0 {
        // SAFETY: The caller guarantees at least capacity writable bytes.
        unsafe { *output = 0 };
    }
}

unsafe fn write_error(output: *mut u8, capacity: usize, error: &str) {
    // SAFETY: The caller of write_error guarantees output and capacity.
    unsafe { write_output(output, capacity, error) }
}

unsafe fn write_output(output: *mut u8, capacity: usize, value: &str) {
    if output.is_null() || capacity == 0 {
        return;
    }
    let bytes = value.as_bytes();
    let copied = bytes.len().min(capacity - 1);
    // SAFETY: The caller guarantees capacity writable bytes, and copied is
    // strictly less than capacity to leave room for the terminator.
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), output, copied);
        *output.add(copied) = 0;
    }
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(script: &[u8]) -> (BscZ3Result, String) {
        let mut error = vec![0_u8; 4096];
        // SAFETY: Both slices remain valid for the complete FFI call.
        let result = unsafe {
            bsc_z3_check_smtlib2(
                script.as_ptr(),
                script.len(),
                error.as_mut_ptr(),
                error.len(),
            )
        };
        let length = error.iter().position(|byte| *byte == 0).unwrap();
        (result, String::from_utf8(error[..length].to_vec()).unwrap())
    }

    #[test]
    fn reports_linked_z3_version() {
        let length = unsafe { bsc_z3_version(ptr::null_mut(), 0) };
        assert!(length > 0);
        let mut output = vec![0_u8; length + 1];
        let reported = unsafe { bsc_z3_version(output.as_mut_ptr(), output.len()) };
        assert_eq!(reported, length);
        let version = String::from_utf8(output[..length].to_vec()).unwrap();
        assert!(version.chars().any(|character| character.is_ascii_digit()));
    }

    #[test]
    fn checks_satisfiable_query() {
        assert_eq!(
            call(b"(declare-const x Int)\n(assert (= x 42))\n").0,
            BscZ3Result::Sat
        );
    }

    #[test]
    fn checks_unsatisfiable_query() {
        assert_eq!(
            call(b"(declare-const x Int)\n(assert (= x 1))\n(assert (= x 2))\n").0,
            BscZ3Result::Unsat
        );
    }

    #[test]
    fn repeated_queries_are_isolated() {
        assert_eq!(call(b"(assert false)\n").0, BscZ3Result::Unsat);
        assert_eq!(call(b"(assert true)\n").0, BscZ3Result::Sat);
    }

    #[test]
    fn concurrent_callers_share_the_worker_safely() {
        let callers = (0..8)
            .map(|index| {
                thread::spawn(move || {
                    let expected = if index % 2 == 0 {
                        BscZ3Result::Sat
                    } else {
                        BscZ3Result::Unsat
                    };
                    let script: &[u8] = if index % 2 == 0 {
                        b"(assert true)\n"
                    } else {
                        b"(assert false)\n"
                    };
                    assert_eq!(call(script).0, expected);
                })
            })
            .collect::<Vec<_>>();
        for caller in callers {
            caller.join().unwrap();
        }
    }

    #[test]
    fn rejects_invalid_utf8_without_unwinding() {
        let (result, error) = call(&[0xff]);
        assert_eq!(result, BscZ3Result::Error);
        assert!(error.contains("not valid UTF-8"));
    }

    #[test]
    fn rejects_null_input_without_unwinding() {
        let mut error = vec![0_u8; 128];
        // SAFETY: Null is intentionally supplied to verify input validation;
        // the error buffer is valid for the complete call.
        let result =
            unsafe { bsc_z3_check_smtlib2(ptr::null(), 0, error.as_mut_ptr(), error.len()) };
        assert_eq!(result, BscZ3Result::Error);
        let length = error.iter().position(|byte| *byte == 0).unwrap();
        assert!(String::from_utf8_lossy(&error[..length]).contains("pointer is null"));
    }
}
