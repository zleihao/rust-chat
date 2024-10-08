use std::collections::HashMap;
use std::io;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};

use crate::server::parse_json;

#[derive(Deserialize, Serialize, Debug)]
pub struct Info {
    #[serde(skip_serializing, skip_deserializing)]
    fd: Option<TcpStream>,
    #[serde(skip_serializing, skip_deserializing)]
    addr: Option<io::Result<SocketAddr>>,
    name: String,
    pass: String,
    #[serde(skip_serializing, skip_deserializing)]
    state: bool, //是否在线
}

pub static mut CLIENT_INFO: LazyLock<Mutex<HashMap<String, Info>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

//欢迎语
pub const STR: &[u8; 24] = b"Welcome to rust chat...\n";
//监听任意地址，端口号默认8080
pub const IP_PORT: &str = "0.0.0.0:8080";

pub fn handler(mut s: TcpStream) {
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
                    //将该用户的信息添加到本地json中
                    if let Err(e) = parse_json::add_to_json_tail("./info.json", v.name.clone(), &v)
                    {
                        println!("{e}");
                    }

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
            }
            Err(e) => {
                let _ = s.write(e.as_bytes());
                continue;
            }
        };
    }
}

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
        "#logout" => unsafe {
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
        },
        "#changepass" => {
            if len != 2 {
                return Err("用法错误\n".to_string());
            }
            if name.is_empty() {
                return Err("你还未登录，不可修改密码\n".to_string());
            }
            if let Err(e) = change_pass(name.clone(), &parts) {
                return Err(e);
            }
            let _ = fd.write("密码修改成功...\n请使用新密码重新登录...\n".as_bytes());
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
                                        \r修改密码:   #changepass <old pass> <new pass>\n\
                                        \r群聊：\t    #broadcast <string>\n\n"
                .to_string();
            let _ = fd.write_all(help_info.as_bytes());
        }
        _ => {
            return Err("未知命令\n".to_string());
        }
    };
    Ok(String::new())
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
            mux.insert(
                buf[1].to_string(),
                Info {
                    fd: Some(fd.try_clone().unwrap()),
                    addr: Some(fd.peer_addr()),
                    name: buf[1].to_string(),
                    pass: "123".to_string(),
                    state: false,
                },
            );
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
                return Err("该用户已经登录了....\n".to_string());
            }
            //该昵称已存在，直接登录
            if v.pass != buf[2].to_string() {
                return Err("密码不对，请重新尝试\n".to_string());
            }

            v.state = true;
            v.fd = Some(fd.try_clone().unwrap());
            return Ok(());
        };
        //该昵称不存在，直接返回错误
        return Err("该昵称不存在，请先注册账户后重试\n".to_string());
    }
}

//修改密码
fn change_pass(name: String, buf: &Vec<&str>) -> anyhow::Result<(), String> {
    unsafe {
        let mut mux = CLIENT_INFO.try_lock().unwrap();
        if let Some(v) = mux.get_mut(&name) {
            if !v.state {
                return Err("你还未登录，无法修改密码\n".to_string());
            }
            if v.pass != buf[1] {
                return Err("旧密码不匹配...\n".to_string());
            }
            //密码修改成功后，退出该用户登录状态
            v.pass = buf[2].to_string();
            v.state = false;
        }
    }

    Ok(())
}
//广播
fn broadcast(fd: TcpStream, name: String, buf: &Vec<&str>) {
    unsafe {
        let map = CLIENT_INFO.try_lock().unwrap();
        for (_, v) in &*map {
            let fd = v.fd.as_ref().unwrap();

            if fd.peer_addr().unwrap() == fd.peer_addr().unwrap() {
                continue;
            }
            let msg = format!("From User {}: {}\n", name, buf[1..].join(" "));
            // 发送消息给所有在线的用户
            if v.state {
                if let Err(e) = fd.try_clone().unwrap().write_all(msg.as_bytes()) {
                    eprintln!("发送消息给 {} 失败: {}", v.name, e);
                }
            }
        }
    }
}
