use std::{fs::File, io::Read, path::PathBuf, sync::Arc};

use mlua::{Lua, LuaSerdeExt};
use serde_json::json;
use tokio::sync::Mutex;

use crate::{configure::PluginConfig, database::DataBaseManager};

mod db;
mod log;

#[derive(Debug)]
pub struct PluginManager {
    available: bool,
    lua: Lua,
    plugin_path: PathBuf,
    pub(crate) config: PluginConfig
}

impl PluginManager {

    pub async fn init(config: &PathBuf) -> crate::Result<Self> {

        let file_config= crate::configure::load_plugin_config(
            &config
        ).unwrap();

        let plugin_path = PathBuf::from(file_config.foundation.path.clone());
        
        let lua = Lua::new();

        let mut available = true;

        if ! plugin_path.is_dir() {
            available = false;
        }

        // 获取加载初始化代码
        if available {
            if plugin_path.join("init.lua").is_file() {
                lua.globals().set("ROOT_PATH", file_config.foundation.path.to_string())?
            }
        }

        Ok(
            Self { lua, available,plugin_path: plugin_path.clone(), config: file_config }
        )
    }

    pub async fn loading(&self, dorea: Arc<Mutex<DataBaseManager>>, current: String) -> crate::Result<()> {

        if self.available {

            let mut f = File::open(self.plugin_path.join("init.lua"))?;

            let mut code = String::new();
        
            let _ = f.read_to_string(&mut code)?;

            self.lua.globals().set("DB_MANAGER", db::PluginDbManager::init(dorea, current).await)?;
            self.lua.globals().set("LOGGER_IN", log::LoggerIn {})?;

            let plugin_table = self.lua.create_table()?;

            for (k, v) in self.config.loader.iter() {
                plugin_table.set(k.to_string(), self.lua.to_value(v).unwrap())?;
            }

            self.lua.globals().set("PLUGIN_LOADER",plugin_table)?;

            self.lua.load(&code).exec()?;

            self.lua.load("MANAGER.call_onload()").exec()?;
        }

        Ok(())
    }

    pub fn call(&self, source: &str) -> crate::Result<()> {

        self.lua.load(source).exec()?;

        Ok(())
    }

    pub fn custom_command(&self, command: &str, mut argument: Vec<String>) -> crate::Result<String> {
        
        argument.remove(0);

        let mut v2t = String::from("{");
        for i in argument {

            let i = i.replace("\"", "\\\"");

            v2t += "\"";
            v2t += &(i + "\", ");
        }
        if v2t.len() > 1 { v2t = v2t[0..v2t.len() - 2].to_string(); }
        v2t += "}";

        let info = json!({
            "argument": v2t
        });

        let command_str = format!(
            "MANAGER.call_command(\"{}\", {})", 
            command,
            info.to_string()
        );

        println!("{}", command_str);

        let v = self.lua.load(&command_str).eval::<String>()?;

        Ok(v)
    }

}