use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::collections::HashMap;
use std::io;
use std::sync::{LazyLock, Mutex};

struct Info {
    fd: TcpStream,
    addr: io::Result<SocketAddr>,
}

//欢迎语
const STR: &[u8; 23] = b"Welcome to rust chat...";
//监听任意地址，端口号默认8080
const IP_PORT: &str = "0.0.0.0:8080";

//使用HashMap保存用户连接信息
static mut CLIENT_INFO: LazyLock<Mutex<std::collections::HashMap<String, Info>>> = LazyLock::new(|| {
    Mutex::new(HashMap::new())
});

fn handler(mut s: TcpStream) {
    let mut buf = [0u8; 1024];

    unsafe {
        let mut c = CLIENT_INFO.lock().unwrap();
        c.insert("1".to_string(), Info { fd: s.try_clone().unwrap(), addr: s.peer_addr() });
    }

    loop {
        let ret = s.read(&mut buf).unwrap_or_else(|e| {
            println!("{e}");
            0
        });

        if ret <= 0 {
            unsafe {
                let c = CLIENT_INFO.lock().unwrap();
                println!("{:?} disconnect",  c.get("1").unwrap().addr);
            }
            let _ = s.shutdown(std::net::Shutdown::Both);
            return;
        }

        //解析指令
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
            }
            Err(e) => println!("{e}"),
        }
    }

    Ok(())
}
