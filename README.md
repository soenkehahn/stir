![ci status badge](https://github.com/soenkehahn/cradle/actions/workflows/ci.yaml/badge.svg)

`cradle` is a library for executing commands in child processes.
Here's an example:

``` rust
use cradle::*;

fn main() {
    let StdoutTrimmed(git_version) = cmd!(%"git --version");
    eprintln!("git version: {}", git_version);
    let (StdoutTrimmed(git_user), Exit(status)) = cmd!(%"git config --get user.name");
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
