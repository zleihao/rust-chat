use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde_json;
use serde_json::{json, Value};

use crate::server::{CLIENT_INFO, Info};

//启动服务器加载用户信息
pub fn start_init(path: &str) -> anyhow::Result<()> {
    println!("开始从本地Json中加载用户信息.....");

    let path = Path::new(path);

    if !path.exists() {
        println!("从本地Json中加载用户信息完成.....");
        return Ok(());
    }
    //打开指定的文件
    let fp = File::open(path)?;

    let reader = BufReader::new(fp);

    let info: HashMap<String, Info> = serde_json::from_reader(reader)?;

    for (k, v) in info {
        unsafe {
            let mut mux = CLIENT_INFO.try_lock().unwrap();
            mux.insert(k, v);
        }
    }

    println!("从本地Json中加载用户信息完成.....");

    Ok(())
}

pub fn add_to_json_tail(path: &str, name: String, info: &Info) -> anyhow::Result<()> {
    let path_obj = Path::new(path);

    let data = if path_obj.exists() {
        //文件存在
        fs::read_to_string(path)?
    } else {
        "{}".to_string()
    };

    let mut json_data: Value = serde_json::from_str(&data)?;

    if let Some(obj) = json_data.as_object_mut() {
        obj.insert(name, json!(info));
    } else {
        println!("Failed to modify the JSON object.");
    }

    // 4. 将更新后的数据结构序列化为 JSON 格式
    let updated_json = serde_json::to_string_pretty(&json_data)?;

    // 保存到文件
    fs::write(path, updated_json)?;

    println!("JSON data updated successfully");

    Ok(())
}
