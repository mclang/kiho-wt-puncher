# Kiho Worktime Puncher

Simple Rust command line application that can be used to make Kiho
worktime `LOGIN` and `LOGOUT` punch lines using Kiho HTTP API.
Running the application first time creates sample TOML configuration file,
path of which is printed out when using verbose (`-v`) mode flag. Thus best
command to start with is something like `kiho-worktime -v get config`.

Command line argument parsing is done using `clap` crate, which handles error
cases and generates `--help` for each command and sub-command automatically.

**Some examples:**
```
$ kiho-worktime get config
$ kiho-worktime get lastest 10 login
$ kiho-worktime start "Things to do, places to be - meetings to attend :/"
$ kiho-worktime -dv stop
$ kiho-worktime --help
```

**TODO**
- More sensible `verbosity` checking with better logging/printing.
- Modularization.
- Improved `String`/`^str` usage.
- Print config path when default sample file is created.

These, plus all the TODO items in the code should probably be added as issues though...

