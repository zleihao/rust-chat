use std::collections::HashMap;
use std::io;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{LazyLock, Mutex};

#[derive(Debug)]
struct Info {
    fd: TcpStream,
    addr: io::Result<SocketAddr>,
    name: String,
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
fn parse(mut fd: TcpStream, name: String, data: &str) -> anyhow::Result<String, String> {
    if 0 == data.len() {
        return Err("\n".to_string());
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
                // let _ = fd.write(e.as_bytes());
                return Err(e);
            }

            let _ = fd.write("昵称设置成功，默认密码为：123\n".as_bytes());
            println!("cmd = {}, name = {}", cmd, parts[1]);
            return Ok(parts[1].to_string());
        }
        "#login" => {
            if len != 2 {
                return Err("用法错误\n".to_string());
            }
            if let Err(e) = login(fd.try_clone().unwrap(), &parts) {
                return Err(e);
            }
            let _ = fd.write("登录成功\n".as_bytes());
            return Ok(parts[1].to_string());
        }
        "#logout" => {
            unsafe {
                let mut mux = CLIENT_INFO.try_lock().unwrap();
                if let Some(v) = mux.get_mut(&name) {
                    if v.state {
                        v.state = false;
                        let _ = fd.write("注销成功\n".as_bytes());
                    } else {
                        return Err("你还未登录不需要注销\n".to_string());
                    }
                } else {
                    return Err("你还未登录不需要注销\n".to_string());
                }
            }
        }
        "#broadcast" | "#b" => {
            if len < 1 {
                return Err("用法错误\n".to_string());
            }

            broadcast(fd.try_clone().unwrap(), name.clone(), &parts);
        }
        "#help" => {
            if len > 0 {
                return Err("用法错误\n".to_string());
            }
            let help_info = "目前聊天室支持以下指令：\n\
                                        \r注册：\t    #setname <user name>\n\
                                        \r登录：\t    #login <user name> <pass>\n\
                                        \r注销：\t    #logout\n\
                                        \r群聊：\t    #broadcast <string>\n\n".to_string();
            let _ = fd.write_all(help_info.as_bytes());
        }
        _ => {
            return Err("未知命令\n".to_string());
        }
    };
    Ok(String::new())
}

fn handler(mut s: TcpStream) {
    let mut buf = [0u8; 1024];
    let mut name: String = String::new();
    loop {
        buf.fill(0);
        let ret = s.read(&mut buf).unwrap_or_else(|e| {
            println!("{e}");
            0
        });

        if ret <= 0 {
            unsafe {
                let mut mux = CLIENT_INFO.lock().unwrap();
                if let Some(v) = mux.get_mut(&name) {
                    v.state = false;
                    println!("用户 {} 已断开聊天室的网", v.name);
                }
            }

            println!("disconnect from {:?}, {name}", s.peer_addr());
            let _ = s.shutdown(std::net::Shutdown::Both);
            return;
        }
        //解析指令
        let data = String::from_utf8_lossy(&mut buf[0..ret]).to_ascii_lowercase();
        let data = data.trim_ascii_start();

        match parse(s.try_clone().unwrap(), name.clone(), &data) {
            Ok(ok) => {
                if !ok.is_empty() {
                    name = ok
                }
            },
            Err(e) => {
                let _ = s.write(e.as_bytes());
                continue;
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
                //为每一个用户创建一个线程
                std::thread::spawn(|| {
                    handler(s);
                });
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
        let mut mux = CLIENT_INFO.lock().unwrap();
        if let Some(_) = mux.get(&buf[1].to_string()) {
            //该昵称已存在，直接返回错误
            return Err("该昵称已存在，请换个昵称试试\n".to_string());
        } else {
            //该昵称不存在
            mux.insert(buf[1].to_string(), Info {
                fd: fd.try_clone().unwrap(),
                addr: fd.peer_addr(),
                name: buf[1].to_string(),
                pass: "123".to_string(),
                state: false,
            });
            println!("{:?}", mux.get(&buf[1].to_string()).unwrap().addr);
        }
    }

    Ok(())
}

//登录
fn login(fd: TcpStream, buf: &Vec<&str>) -> anyhow::Result<(), String> {
    //判断是否已经有该昵称
    unsafe {
        let mut mux = CLIENT_INFO.lock().unwrap();
        if let Some(v) = mux.get_mut(&buf[1].to_string()) {
            if v.state {
                return Err("该用户已经登录了....".to_string());
            }
            //该昵称已存在，直接登录
            if v.pass != buf[2].to_string() {
                return Err("密码不对，请重新尝试\n".to_string());
            }

            v.state = true;
            v.fd = fd.try_clone().unwrap();
            return Ok(());
        };
        //该昵称不存在，直接返回错误
        return Err("该昵称不存在，请先注册账户后重试\n".to_string());
    }
}

//广播
fn broadcast(fd: TcpStream, name: String, buf: &Vec<&str>) {
    unsafe {
        let map = CLIENT_INFO.try_lock().unwrap();
        for (_, v) in &*map {
            if v.fd.peer_addr().unwrap() == fd.peer_addr().unwrap() {
                continue;
            }
            let msg = format!("From User {}: {}\n", name, buf[1..].join(" "));
            // 发送消息给所有在线的用户
            if v.state {
                if let Err(e) = v.fd.try_clone().unwrap().write_all(msg.as_bytes()) {
                    eprintln!("发送消息给 {} 失败: {}", v.name, e);
                }
            }
        }
    }
}
