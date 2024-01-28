
pub const FAILURE_STR: &str = "error: the following required arguments were not provided:
  --script-folder <SCRIPT_FOLDER>

Usage: rusty-hooks --script-folder <SCRIPT_FOLDER>

For more information, try \'--help\'.
";
pub const INVALID_SCRIPT_FOLDER_REGEX:&str = r#"\n(?<debug_level>level=(INFO|TRACE|ERROR|DEBUG)) | (?<message>message="io error while parsing command line args: `No such file or directory \(os error 2\)`") | (?<src>src=.*)"#;
pub const LOGGING_REGEX:&str = r#"\n(?<debug_level>level=(INFO|TRACE|ERROR|DEBUG)) | (?<message>message=".*") | (?<src>src=.*)"#;
pub const WATCH_PATH_REGEX:&str = r#"\n(?<debug_level>level=(INFO|TRACE|ERROR|DEBUG)) | (?<message>message="now watching path: \/media\/wd3\/programming\/rusty-hooks\/tests\/files\/watch_location") | (?<src>src=.*)"#;
