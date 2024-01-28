
pub const FAILURE_STR: &str = "error: the following required arguments were not provided:
  --script-folder <SCRIPT_FOLDER>

Usage: rusty-hooks --script-folder <SCRIPT_FOLDER>

For more information, try \'--help\'.
";
pub const INVALID_SCRIPT_FOLDER_REGEX:&str = r#"\n(?<timestamp>ts=(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2}\.[0-9]*)\+(\d{2}:\d{2})) (?<debug_level>level=(INFO|TRACE|ERROR|DEBUG)) (?<message>message="io error while parsing command line args: `No such file or directory \(os error 2\)`") (?<src>src=.*) (?<pid>pid=[0-9]*)"#;
pub const LOGGING_REGEX:&str = r#"\n(?<timestamp>ts=(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2}\.[0-9]*)\+(\d{2}:\d{2})) (?<debug_level>level=(INFO|TRACE|ERROR|DEBUG)) (?<message>message=".*") (?<src>src=.*) (?<pid>pid=[0-9]*)"#;
pub const WATCH_PATH_REGEX:&str = r#"\n(?<timestamp>ts=(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2}\.[0-9]*)\+(\d{2}:\d{2})) (?<debug_level>level=(INFO|TRACE|ERROR|DEBUG)) (?<message>message="now watching path: \/media\/wd3\/programming\/rusty-hooks\/tests\/files\/watch_location") (?<src>src=.*) (?<pid>pid=[0-9]*)"#;
