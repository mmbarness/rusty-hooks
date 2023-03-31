## Rusty-Hooks

### What is this
I found myself needing a way to continually execute arbitrary bash scripts based on filesystem events at certain locations, and didn't really come across any good, simple options. So that's what this is meant to do. If you need something to run when something else happens, somewhere, you might find this useful. My own usecase is when media is downloaded to my server, I want that media to be processed automatically with a command line application, but I've kept it abstract enough to be pretty generally applicable.

### Installation

This runs on rust nightly for the moment.

### How to run it

At the moment you'll have to clone the repo and build the binaries, then run them. The application expects the scripts to be local to wherever the binaries are run from, in a folder called 'user_scripts', and within that folder expects the bash scripts along with a 'scripts_config.json'. That file should have an array structure of objects that look like this: 
```
    {
        "event_triggers": ["EventKinds"],
        "file_name": "whatever.sh",
        "watch_path": "path/that/should/be/watched",
        "run_delay": u8, in case you want to bake an extra delay from the last event to the script execution
    },
```
From here, you would execute the binary and provide the arguments like this `--watch-path path/that/should/be/watched`. You can also pass a debug level like so: --log_level debug|info|error. The log level defaults to error.

[![unit tests](https://github.com/mmbarness/rusty-hooks/actions/workflows/test.yml/badge.svg)](https://github.com/mmbarness/rusty-hooks/actions/workflows/test.yml)