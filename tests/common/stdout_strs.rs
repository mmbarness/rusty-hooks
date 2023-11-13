
pub const FAILURE_STR: &str = "error: the following required arguments were not provided:
  --script-folder <SCRIPT_FOLDER>

Usage: rusty-hooks --script-folder <SCRIPT_FOLDER>

For more information, try \'--help\'.
";
pub const INVALID_SCRIPT_FOLDER_REGEX:&str = r#"\n(?<timestamp>ts=(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2}\.[0-9]*)\+(\d{2}:\d{2})) (?<debug_level>level=(INFO|TRACE|ERROR|DEBUG)) (?<message>message="io error while parsing command line args: `No such file or directory \(os error 2\)`") (?<src>src=.*) (?<pid>pid=[0-9]*)"#;
pub const LOGGING_REGEX:&str = r#"\n(?<timestamp>ts=(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2}\.[0-9]*)\+(\d{2}:\d{2})) (?<debug_level>level=(INFO|TRACE|ERROR|DEBUG)) (?<message>message=".*") (?<src>src=.*) (?<pid>pid=[0-9]*)"#;
pub const WATCH_PATH_REGEX:&str = r#"\n(?<timestamp>ts=(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2}\.[0-9]*)\+(\d{2}:\d{2})) (?<debug_level>level=(INFO|TRACE|ERROR|DEBUG)) (?<message>message="now watching path: \/media\/wd3\/programming\/rusty-hooks\/tests\/files\/watch_location") (?<src>src=.*) (?<pid>pid=[0-9]*)"#;

pub const SUCCESS_STR: &str = r#"ts=2023-10-08T19:38:43.697974722+00:00 level=INFO message="starting rusty hooks...." src=src/main.rs pid=885731
ts=2023-10-08T19:38:43.698014918+00:00 level=DEBUG message="config path: /media/wd3/programming/rusty-hooks/tests/files/scripts" src=src/utilities/cli_args.rs pid=885731
ts=2023-10-08T19:38:43.698051576+00:00 level=DEBUG message="default path to lockfile is: /home/mmbarnes/rusty-hooks/rusty-hooks.pid" src=src/utilities/set_process_lockfile.rs pid=885731
ts=2023-10-08T19:38:43.698177103+00:00 level=DEBUG message="all paths validated" src=src/scripts/load.rs pid=885731
ts=2023-10-08T19:38:43.698191965+00:00 level=DEBUG message="path: /media/wd3/programming/rusty-hooks/tests/files/scripts/test_script.sh" src=src/utilities/traits.rs pid=885731
ts=2023-10-08T19:38:43.698201876+00:00 level=DEBUG message="1 scripts found that match provided watch path" src=src/scripts/load.rs pid=885731
ts=2023-10-08T19:38:43.698207834+00:00 level=DEBUG message="deciding whether to insert script "\"test_script.sh\""" src=src/scripts/load.rs pid=885731
ts=2023-10-08T19:38:43.698274881+00:00 level=INFO message="now watching path: /media/wd3/programming/rusty-hooks/tests/files/watch_location" src=src/watcher/init.rs pid=885731
ts=2023-10-08T19:38:43.698287265+00:00 level=TRACE message="registering event source with poller: token=Token(0), interests=READABLE" src=/home/mmbarnes/.cargo/registry/src/index.crates.io-6f17d22bba15001f/mio-0.8.8/src/poll.rs pid=885731
ts=2023-10-08T19:38:43.698400842+00:00 level=DEBUG message="now awaiting events, subscriptions, and unsubscriptions task" src=src/watcher/init.rs pid=885731
ts=2023-10-08T19:38:43.698440456+00:00 level=DEBUG message="spawned subscribe thread" src=src/watcher/path_subscriber.rs pid=885731
ts=2023-10-08T19:38:43.698494211+00:00 level=DEBUG message="spawned unsubscribe thread" src=src/watcher/path_subscriber.rs pid=885731
ts=2023-10-08T19:38:43.698529155+00:00 level=DEBUG message="spawned event watching thread" src=src/watcher/watch_events.rs pid=885731
"#;
