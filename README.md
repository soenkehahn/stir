[![ci status badge](https://github.com/soenkehahn/cradle/actions/workflows/ci.yaml/badge.svg)](https://github.com/soenkehahn/cradle/actions?query=branch%3Amaster)
[![crates.io](https://img.shields.io/crates/v/cradle.svg)](https://crates.io/crates/cradle)
[![docs](https://docs.rs/cradle/badge.svg)](https://docs.rs/cradle)

`cradle` is a library for executing commands in child processes.
Here's an example:

``` rust
use cradle::prelude::*;

fn main() {
    // output git version
    run!(%"git --version");
    // output configured git user
    let (StdoutTrimmed(git_user), Status(status)) = run_output!(%"git config --get user.name");
    if status.success() {
        eprintln!("git user: {}", git_user);
    } else {
        eprintln!("git user not configured");
    }
}
```

For comprehensive documentation, head over to
[docs.rs/cradle](https://docs.rs/cradle/latest/cradle/).

## MSRV
The minimal supported rust version is `0.41`.
