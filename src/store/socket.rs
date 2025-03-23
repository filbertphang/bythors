use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn read_bytes_from_socket_to_str<T>(stream: &mut T, len: usize) -> String
where
    T: AsyncReadExt + Unpin,
{
    // read actual message into buf
    let mut buf = vec![0u8; len];
    stream
        .read_exact(&mut buf)
        .await
        .expect("should be able to read msg from stream");

    // parse message into a Request
    let msg = String::from_utf8(buf).expect("should be able to convert msg into string");
    msg
}

pub async fn write_str_to_socket<T>(stream: &mut T, msg: String)
where
    T: AsyncWriteExt + Unpin,
{
    let bytes = msg.into_bytes();

    let len: u32 = bytes
        .len()
        .try_into()
        .expect("message length should fit into a u32");

    // write message to tcp stream
    stream
        .write_u32_le(len)
        .await
        .expect("should be able to write msg length to stream");
    stream
        .write(&bytes)
        .await
        .expect("should be able to write message to stream");
}
