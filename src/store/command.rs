#[derive(Debug, Clone)]
pub enum Request {
    Get { key: String },
    Put { key: String, val: String },
}

#[derive(Debug, Clone)]
pub enum Response {
    GetR { key: String, val: Option<String> },
    NotLeader,
}

pub fn parse_request(raw_input: String) -> Option<Request> {
    println!("received: {raw_input}");
    // input format:
    // <client id> <request id> <cmd> <arg1> <arg2>
    //     0            1         2     3      4
    let res: Vec<&str> = raw_input.split(' ').collect();

    if res.len() < 4 {
        return None;
    }

    let command = res[2].trim().to_uppercase();
    match command.as_str() {
        "GET" => Some(res).filter(|v| v.len() >= 4).map(|v| Request::Get {
            key: v[3].trim().to_string(),
        }),
        "PUT" => Some(res).filter(|v| v.len() >= 5).map(|v| Request::Put {
            key: v[3].trim().to_string(),
            val: v[4].trim().to_string(),
        }),
        _ => None,
    }
}

pub fn pack_response(res: Response) -> Vec<u8> {
    let res_str = match res {
        Response::GetR { key, val } => {
            let val_s = val.unwrap_or(String::from("-"));
            format!("Response 0 {key} {val_s} -")
        }
        Response::NotLeader => String::from("NotLeader"),
    };

    res_str.into_bytes()
}
