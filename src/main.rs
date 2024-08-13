use std::io::Write;
use std::net::TcpListener;

mod server;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listen = TcpListener::bind(server::IP_PORT)?;

    for sfd in listen.incoming() {
        match sfd {
            Ok(mut s) => {
                println!("Connect from {}", s.peer_addr()?);
                let _ = s.write(server::STR);
                //为每一个用户创建一个线程
                std::thread::spawn(|| {
                    server::handler(s);
                });
            }
            Err(e) => println!("{e}"),
        }
    }

    Ok(())
}
