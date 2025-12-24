use std::error::Error;
use vergen_gitcl::{BuildBuilder, Emitter, GitclBuilder};

fn main() -> Result<(), Box<dyn Error>> {
    // 检查是否有自定义版本环境变量
    if let Ok(custom_describe) = std::env::var("CUSTOM_GIT_DESCRIBE") {
        // 使用自定义版本
        println!("cargo: rustc-env=VERGEN_GIT_DESCRIBE={}", custom_describe);
        
        // 仍然需要设置 build 信息
        let build = BuildBuilder::all_build()?;
        Emitter::default()
            .add_instructions(&build)?
            .emit()?;
    } else {
        // 使用原有的 vergen 逻辑
        let git = GitclBuilder::default()
            .all()
            .describe(true, true, None)
            .build()?;
        let build = BuildBuilder::all_build()?;
        Emitter::default()
            .add_instructions(&build)?
            .add_instructions(&git)?
            .emit()?;
    }
    Ok(())
}