use std::collections::HashMap;
use std::io;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{LazyLock, Mutex};

#[derive(Debug)]
struct Info {
    fd: TcpStream,
    addr: io::Result<SocketAddr>,
    pass: String,
    state: bool, //是否在线
}

//欢迎语
const STR: &[u8; 24] = b"Welcome to rust chat...\n";
//监听任意地址，端口号默认8080
const IP_PORT: &str = "0.0.0.0:8080";

//使用HashMap保存用户连接信息
static mut CLIENT_INFO: LazyLock<Mutex<std::collections::HashMap<String, Info>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

//解析用户命令
fn parse(mut fd: TcpStream, data: &str) -> anyhow::Result<(), String> {
    if 0 == data.len() {
        return Err("all space".to_string());
    }

    //判断是否满足指令格式
    if !data.starts_with('#') {
        return Err("命令格式有误\n".to_string());
    }

    let parts = data.split_ascii_whitespace().collect::<Vec<&str>>();
    //得到执行命令
    let (cmd, len) = (parts[0], parts[1..].len());
    match cmd {
        "#setname" => {
            if len != 1 {
                return Err("用法错误\n".to_string());
            }
            if let Err(e) = set_name(fd.try_clone().unwrap(), &parts) {
                let _ = fd.write(e.as_bytes());
            } else {
                let _ = fd.write("昵称设置成功，默认密码为：123\n".as_bytes());
            }
            println!("cmd = {}, name = {}", cmd, parts[1]);
        }
        "#login" => {
            if len != 2 {
                return Err("用法错误\n".to_string());
            }
            println!("该命令是登录");
        }
        _ => {
            return Err("未知命令\n".to_string());
        }
    };
    Ok(())
}

fn handler(mut s: TcpStream) {
    let mut buf = [0u8; 1024];
    loop {
        buf.fill(0);
        let ret = s.read(&mut buf).unwrap_or_else(|e| {
            println!("{e}");
            0
        });

        if ret <= 0 {
            let _ = s.shutdown(std::net::Shutdown::Both);
            return;
        }
        //解析指令
        let data = String::from_utf8_lossy(&mut buf[0..ret]).to_ascii_lowercase();
        let data = data.trim_ascii_start();

        if let Err(e) = parse(s.try_clone().unwrap(), &data) {
            match e.as_str() {
                "all space" => continue,
                _ => {
                    println!("{e}");
                    if let Err(_) = s.write(e.as_bytes()) {
                        continue;
                    }
                },
            }
        };
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

//处理对应命令
fn set_name(fd: TcpStream, buf: &Vec<&str>) -> anyhow::Result<(), String> {
    //判断是否已经有该昵称
    unsafe {
        let mut mux = CLIENT_INFO.try_lock().unwrap();
        if let Some(_) = mux.get(&buf[0].to_string()) {
            //该昵称已存在，直接返回错误
            return Err("该昵称已存在，请换个昵称试试\n".to_string());
        } else {
            //该昵称不存在
            mux.insert(buf[0].to_string(), Info {
                fd: fd.try_clone().unwrap(),
                addr: fd.peer_addr(),
                pass: "123".to_string(),
                state: false,
            });
            println!("{:?}", mux.get(&buf[0].to_string()).unwrap().addr);
        }
    }

    Ok(())
}