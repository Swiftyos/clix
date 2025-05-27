use clix::commands::Command;
use clix::storage::Storage;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use test_context::{AsyncTestContext, test_context};

struct PerfContext {
    temp_dir: PathBuf,
    storage: Storage,
}

impl AsyncTestContext for PerfContext {
    fn setup<'a>() -> std::pin::Pin<Box<dyn std::future::Future<Output = Self> + Send + 'a>> {
        Box::pin(async {
            let temp_dir = std::env::temp_dir().join("clix_perf_test").join(format!(
                "test_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_micros()
            ));
            fs::create_dir_all(&temp_dir).unwrap();
            let clix_dir = temp_dir.join(".clix");
            fs::create_dir_all(&clix_dir).unwrap();
            let commands_file = clix_dir.join("commands.json");
            fs::write(&commands_file, r#"{"commands":{},"workflows":{}}"#).unwrap();
            unsafe {
                env::set_var("HOME", &temp_dir);
            }
            let storage = Storage::new().unwrap();
            PerfContext { temp_dir, storage }
        })
    }

    fn teardown<'a>(self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            fs::remove_dir_all(&self.temp_dir).unwrap_or_default();
        })
    }
}

#[test_context(PerfContext)]
#[tokio::test]
async fn benchmark_add_commands(ctx: &mut PerfContext) {
    let start = Instant::now();
    for i in 0..100u32 {
        let cmd = Command::new(
            format!("cmd-{}", i),
            "benchmark".to_string(),
            "echo benchmark".to_string(),
            vec![],
        );
        ctx.storage.add_command(cmd).unwrap();
    }
    let duration = start.elapsed();
    println!("Added 100 commands in {:?}", duration);
    assert!(
        duration.as_secs_f32() < 1.0,
        "adding commands took too long: {:?}",
        duration
    );
}
