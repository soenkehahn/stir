use crate::{config::Config, error::Error, RunResult};
use std::{process::ExitStatus, sync::Arc};

/// All possible return types of [`cmd!`], [`cmd_unit!`] or
/// [`cmd_result!`] must implement this trait.
/// This return-type polymorphism makes cradle very flexible.
/// For example, if you want to capture what a command writes
/// to `stdout` you can do that using [`StdoutUntrimmed`]:
///
/// ```
/// use cradle::prelude::*;
///
/// let StdoutUntrimmed(output) = cmd!(%"echo foo");
/// assert_eq!(output, "foo\n");
/// ```
///
/// But if instead you want to capture the command's [`ExitStatus`],
/// you can use [`Status`]:
///
/// ```
/// use cradle::prelude::*;
///
/// let Status(exit_status) = cmd!("false");
/// assert_eq!(exit_status.code(), Some(1));
/// ```
///
/// For documentation on what all the possible return types do,
/// see the documentation for the individual impls of [`Output`].
/// Here's a non-exhaustive list of the more commonly used return types to get you started:
///
/// - [`()`]: In case you don't want to capture anything. See also [`cmd_unit`].
/// - To capture output streams:
///   - [`StdoutTrimmed`]: To capture `stdout`, trimmed of whitespace.
///   - [`StdoutUntrimmed`]: To capture `stdout` untrimmed.
///   - [`Stderr`]: To capture `stderr`.
/// - [`Status`]: To capture the command's [`ExitStatus`].
///
/// Also, [`Output`] is implemented for tuples.
/// You can use this to combine multiple return types that implement [`Output`].
/// The following code for example retrieves the command's [`ExitStatus`]
/// **and** what it writes to `stdout`:
///
/// ```
/// use cradle::prelude::*;
///
/// let (Status(exit_status), StdoutUntrimmed(stdout)) = cmd!(%"echo foo");
/// assert!(exit_status.success());
/// assert_eq!(stdout, "foo\n");
/// ```
///
/// [`()`]: trait.Output.html#impl-Output-for-()
pub trait Output: Sized {
    #[doc(hidden)]
    fn configure(config: &mut Config);

    #[doc(hidden)]
    fn from_run_result(config: &Config, result: Result<RunResult, Error>) -> Result<Self, Error>;
}

/// Use this when you don't need any result from the child process.
///
/// ```
/// # let temp_dir = tempfile::TempDir::new().unwrap();
/// # std::env::set_current_dir(&temp_dir).unwrap();
/// use cradle::prelude::*;
///
/// let () = cmd!(%"touch ./foo");
/// ```
///
/// Since [`cmd!`] (and [`cmd_result`]) use return type polymorphism,
/// you have to make sure the compiler can figure out which return type you want to use.
/// In this example that happens through the `let () =`.
/// So you can't just omit that.
///
/// See also [`cmd_unit!`] for a more convenient way to use `()` as the return type.
impl Output for () {
    #[doc(hidden)]
    fn configure(_config: &mut Config) {}

    #[doc(hidden)]
    fn from_run_result(_config: &Config, result: Result<RunResult, Error>) -> Result<Self, Error> {
        result?;
        Ok(())
    }
}

macro_rules! tuple_impl {
    ($($generics:ident,)+) => {
        impl<$($generics),+> Output for ($($generics,)+)
        where
            $($generics: Output,)+
        {
            #[doc(hidden)]
            fn configure(config: &mut Config) {
                $(<$generics as Output>::configure(config);)+
            }

            #[doc(hidden)]
            fn from_run_result(config: &Config, result: Result<RunResult, Error>) -> Result<Self, Error> {
                Ok((
                    $(<$generics as Output>::from_run_result(config, result.clone())?,)+
                ))
            }
        }
    };
}

tuple_impl!(A,);
tuple_impl!(A, B,);
tuple_impl!(A, B, C,);
tuple_impl!(A, B, C, D,);
tuple_impl!(A, B, C, D, E,);
tuple_impl!(A, B, C, D, E, F,);

/// Returns what the child process writes to `stdout`, interpreted as utf-8,
/// collected into a string, trimmed of leading and trailing whitespace.
/// This also suppresses output of the child's `stdout`
/// to the parent's `stdout`. (Which would be the default when not using [`StdoutTrimmed`]
/// as the return value.)
///
/// It's recommended to pattern-match to get to the inner [`String`].
/// This will make sure that the return type can be inferred.
/// Here's an example:
///
/// ```
/// use std::path::Path;
/// use cradle::prelude::*;
///
/// # #[cfg(unix)]
/// # {
/// let StdoutTrimmed(output) = cmd!(%"which ls");
/// assert!(Path::new(&output).exists());
/// # }
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct StdoutTrimmed(pub String);

impl Output for StdoutTrimmed {
    #[doc(hidden)]
    fn configure(config: &mut Config) {
        StdoutUntrimmed::configure(config);
    }

    #[doc(hidden)]
    fn from_run_result(config: &Config, result: Result<RunResult, Error>) -> Result<Self, Error> {
        let StdoutUntrimmed(stdout) = StdoutUntrimmed::from_run_result(config, result)?;
        Ok(StdoutTrimmed(stdout.trim().to_owned()))
    }
}

/// Same as [`StdoutTrimmed`], but does not trim whitespace from the output:
///
/// ```
/// use cradle::prelude::*;
///
/// let StdoutUntrimmed(output) = cmd!(%"echo foo");
/// assert_eq!(output, "foo\n");
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct StdoutUntrimmed(pub String);

impl Output for StdoutUntrimmed {
    #[doc(hidden)]
    fn configure(config: &mut Config) {
        config.relay_stdout = false;
    }

    #[doc(hidden)]
    fn from_run_result(config: &Config, result: Result<RunResult, Error>) -> Result<Self, Error> {
        let result = result?;
        Ok(StdoutUntrimmed(String::from_utf8(result.stdout).map_err(
            |source| Error::InvalidUtf8ToStdout {
                full_command: config.full_command(),
                source: Arc::new(source),
            },
        )?))
    }
}

/// [`Stderr`] allows to capture the `stderr` of a child process:
///
/// ```
/// use cradle::prelude::*;
///
/// // (`Status` is used here to suppress panics caused by `ls`
/// // terminating with a non-zero exit code.)
/// let (Stderr(stderr), Status(_)) = cmd!(%"ls does-not-exist");
/// assert!(stderr.contains("No such file or directory"));
/// ```
///
/// This assumes that the output written to `stderr` is encoded
/// as utf-8, and will error otherwise.
///
/// By default, what is written to `stderr` by the child process
/// is relayed to the parent's `stderr`. However, when [`Stderr`]
/// is used, this is switched off.
#[derive(Debug)]
pub struct Stderr(pub String);

impl Output for Stderr {
    #[doc(hidden)]
    fn configure(config: &mut Config) {
        config.relay_stderr = false;
    }

    #[doc(hidden)]
    fn from_run_result(config: &Config, result: Result<RunResult, Error>) -> Result<Self, Error> {
        Ok(Stderr(String::from_utf8(result?.stderr).map_err(
            |source| Error::InvalidUtf8ToStderr {
                full_command: config.full_command(),
                source: Arc::new(source),
            },
        )?))
    }
}

/// Use [`Status`] as the return type for [`cmd!`] to retrieve the
/// [`ExitStatus`] of the child process:
///
/// ```
/// use cradle::prelude::*;
///
/// let Status(exit_status) = cmd!(%"echo foo");
/// assert!(exit_status.success());
/// ```
///
/// Also, when using [`Status`], non-zero exit codes won't
/// result in neither a panic nor a [`std::result::Result::Err`]:
///
/// ```
/// use cradle::prelude::*;
///
/// let Status(exit_status) = cmd!("false");
/// assert_eq!(exit_status.code(), Some(1));
/// let result: Result<Status, cradle::Error> = cmd_result!("false");
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap().0.code(), Some(1));
/// ```
///
/// Also see the
/// [section about error handling](index.html#error-handling) in
/// the module documentation.
pub struct Status(pub ExitStatus);

impl Output for Status {
    #[doc(hidden)]
    fn configure(config: &mut Config) {
        config.error_on_non_zero_exit_code = false;
    }

    #[doc(hidden)]
    fn from_run_result(_config: &Config, result: Result<RunResult, Error>) -> Result<Self, Error> {
        Ok(Status(result?.exit_status))
    }
}

/// Using [`bool`] as the return type for [`cmd!`] will return `true` if
/// the command returned successfully, and `false` otherwise:
///
/// ```
/// use cradle::prelude::*;
///
/// if !cmd!(%"which cargo") {
///     panic!("Cargo is not installed!");
/// }
/// ```
///
/// Also, when using [`bool`], non-zero exit codes will not result in a panic
/// or [`std::result::Result::Err`]:
///
/// ```
/// use cradle::prelude::*;
///
/// let success: bool = cmd!("false");
/// assert!(!success);
/// let result: Result<bool, cradle::Error> = cmd_result!("false");
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap(), false);
/// ```
///
/// Also see the
/// [section about error handling](index.html#error-handling) in
/// the module documentation.
impl Output for bool {
    #[doc(hidden)]
    fn configure(config: &mut Config) {
        config.error_on_non_zero_exit_code = false;
    }

    #[doc(hidden)]
    fn from_run_result(_config: &Config, result: Result<RunResult, Error>) -> Result<Self, Error> {
        Ok(result?.exit_status.success())
    }
}
