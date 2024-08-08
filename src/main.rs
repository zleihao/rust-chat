use std::net::{TcpListener, TcpStream};
use std::io::{Write, Read};

const STR: &[u8; 23] = b"Welcome to rust chat...";
const IP_PORT: &str = "0.0.0.0:8080";

fn handler(mut s: TcpStream) {
    let mut buf = [0u8;1024];

    loop {
        let ret = match s.read(&mut buf) {
            Ok(n) => n,
            Err(e) => {
                println!("{e}");
                0
            }, 
        };

        if ret <= 0 {
            println!("{:?} disconnect", s.peer_addr());
            let _ = s.shutdown(std::net::Shutdown::Both);
            return;
        }

        let _ = s.write(&buf[0..ret]);
        let _ = std::io::stdout().write(&buf[0..ret]);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listen = TcpListener::bind(IP_PORT)?;

    for sfd in listen.incoming() {
        match sfd {
            Ok(mut s) => {
                println!("Connect from {}", s.peer_addr()?);
                let _ = s.write(STR);

                let tid = std::thread::spawn(|| {
                    handler(s);
                });
                let _ = tid.join();
            },
            Err(e) => println!("{e}"),
        }
    }

    Ok(())
}