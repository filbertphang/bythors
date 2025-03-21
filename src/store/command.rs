enum Command {
    Get { key: String },
    Put { key: String, val: String },
}

fn parse_command(raw_input: String) -> Option<Command> {
    let res: Vec<&str> = raw_input.splitn(3, ' ').collect();

    if res.len() < 2 {
        return None;
    }

    let command = res[0].trim().to_uppercase();
    match command.as_str() {
        "GET" => Some(res).filter(|v| v.len() == 2).map(|v| Command::Get {
            key: v[1].trim().to_string(),
        }),
        "PUT" => Some(res).filter(|v| v.len() == 3).map(|v| Command::Put {
            key: v[1].trim().to_string(),
            val: v[2].trim().to_string(),
        }),
        _ => None,
    }
}
