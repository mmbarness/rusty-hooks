## Rusty-Hooks
[![Build](https://github.com/mmbarness/rusty-hooks/actions/workflows/on_release.yml/badge.svg?event=release)](https://github.com/mmbarness/rusty-hooks/actions/workflows/on_release.yml)
![Tests](https://github.com/mmbarness/rusty-hooks/actions/workflows/test.yml/badge.svg)
[![Compile](https://github.com/mmbarness/rusty-hooks/actions/workflows/check.yml/badge.svg?event=release)](https://github.com/mmbarness/rusty-hooks/actions/workflows/check.yml)

### What is this
I found myself needing a way to continually execute arbitrary bash scripts based on filesystem events at certain locations, and didn't really come across any good, simple options. So that's what this is meant to do. If you need something to run when something else happens, somewhere, you might find this useful. My own usecase is when media is downloaded to my server, I want that media to be processed automatically with filebot, a command line application, but I've kept it abstract enough to be pretty generally applicable. Because it's multithreaded it can efficiently watch multiple directories.

### Installation

Binaries are available for each release, in the releases section of this repo.

### How to run it

The application requires a folder of scripts with a configuration yaml file. A schema is at root to provide guidance, but here's an example: 
```yaml
# yaml-language-server: $schema=../scripts.schema.json
scripts:
- name: "music"
  description: "run filebot against music download folder"
  watch_path: /path/to/music_download_location
  file_name: ingest_music.sh
  enabled: true
  run_delay: 0
  event_triggers: ["Modify"]
  dependencies: []
- name: "movies"
  description: "run filebot against movies download folder"
  file_name: ingest_movies.sh
  watch_path: /path/to/movie_download_location
  enabled: true
  run_delay: 0
  event_triggers: ["Modify"]
  dependencies: []
- name: "tv"
  description: "run filebot against tv download folder"
  file_name: ingest_tv.sh
  watch_path: /path/to/tv_download_location
  enabled: true
  run_delay: 0
  event_triggers: ["Modify"]
  dependencies: []
```

Every watch path provided in the json will be picked up by rusty-hooks, unless `enabled` is false, of course.

Tell rusty-hooks where to find the folder by passing it to the cli, like `--script-config /home/<username>/scripts/`. The config file and the scripts need to be in the same folder. You can also pass a debug level like so: `--log-level debug`. The log level defaults to error.

The event triggers rely on the [notify](https://docs.rs/crate/notify/latest) crate's EventKind structs, and at this point rusty-hooks cannot parse EventKind subcategories e.g. Modify(Name(To)). If you provide an event trigger of Modify, every kind of Modify event will match.

Here's an example command, with output.

```
user@machine:~/dir/rusty-hooks$ target/release/rusty-hooks --script-config ./scripts --log-level debug
[2023-03-31T13:17:09Z INFO  rusty_hooks::logger::structs] log level set to info
[2023-03-31T13:17:09Z INFO  rusty_hooks::logger::structs] log level set to debug
[2023-03-31T13:17:09Z INFO  rusty_hooks::logger::structs] log level set to error
[2023-03-31T13:17:09Z DEBUG rusty_hooks::logger::debug] path: ./scripts/ingest_music.sh
[2023-03-31T13:17:09Z DEBUG rusty_hooks::logger::debug] path: ./scripts/ingest_movies.sh
[2023-03-31T13:17:09Z DEBUG rusty_hooks::logger::debug] 1 scripts found that match provided watch path
[2023-03-31T13:17:09Z DEBUG rusty_hooks::scripts::load] deciding whether to insert script Ok("\"ingest_music.sh\"")
[2023-03-31T13:17:09Z DEBUG rusty_hooks::logger::debug] spawned subscribe thread
[2023-03-31T13:17:09Z DEBUG rusty_hooks::logger::debug] spawned event watching thread
[2023-03-31T13:17:09Z DEBUG rusty_hooks::logger::debug] spawned unsubscribe thread
```
### Logging
If you're running this on Linux, logs will be written to home/*ur_username*/rusty-hooks/logs/rusty-hooks.log. On mac, they write to ~/Library/rusty-hooks/logs/.
