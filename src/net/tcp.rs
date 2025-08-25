use std::{
    io,
    net::{TcpListener, TcpStream},
    thread,
};

use crate::errors::MokErrors;

pub fn start() -> io::Result<TcpListener> {
    let listener = TcpListener::bind("0.0.0.0:8000")?;
    let mut conns = vec![];
    loop {
        let (strem, addr) = listener.accept()?;
        conns.push(thread::spawn(move || {
            if let Err(e) = handle_stream(strem) {
                eprintln!("client error: {}", e)
            }
        }));
    }
}

fn handle_stream(strem: TcpStream) -> Result<(), MokErrors> {
    Ok(())
}
