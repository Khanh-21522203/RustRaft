pub struct Command {
    kind: i64,

    key: String,
    value: String,

    compare_value: String,
    result_value: String,
    result_found: String,

    service_id: i64,

    client_id: i64,
    request_id: i64,

    is_duplicate: bool,
}

pub enum CommandType {
    CommandInvalid = 0,
    CommandGet = 1,
    CommandPut = 2,
    CommandAppend = 3,
    CommandCompareAndSet = 4,
}

impl CommandType {
    fn as_str(&self) -> &'static str {
        match self {
            CommandType::CommandInvalid => "invalid",
            CommandType::CommandGet     => "get",
            CommandType::CommandPut     => "put",
            CommandType::CommandAppend  => "append",
            CommandType::CommandCompareAndSet     => "cas",
        }
    }
}