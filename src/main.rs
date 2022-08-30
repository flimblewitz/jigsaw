use tonic::transport::Server;

mod jigsaw_instance;
use jigsaw_instance::JigsawInstance;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // todo: replace this hardcoded config with an environment variable
    // let config: JigsawConfig = serde_json::from_str(&env::var("CONFIG")?).unwrap();
    let json = r#"{
  "service_name": "default_jigsaw",
  "grpc_method_a": {
    "tracing_name": "do_a_barrel_roll",
    "steps": [
      {
        "Action": {
          "Sleep": {
            "tracing_name": "listen_to_peppy",
            "duration_ms": 1000
          }
        }
      },
      {
        "ConcurrentActions": [
          {
            "Sleep": {
              "tracing_name": "press L",
              "duration_ms": 2000
            }
          },
          {
            "Sleep": {
              "tracing_name": "press R",
              "duration_ms": 1500
            }
          },
          {
            "Function": {
              "tracing_name": "accidentally unplug controller",
              "steps": [
                {
                  "Action": {
                    "Sleep": {
                      "tracing_name": "instinctually yank the controller to physically pull the arwing into a barrel roll",
                      "duration_ms": 1000
                    }
                  }
                },
                {
                  "Action": {
                    "Sleep": {
                      "tracing_name": "unplug the cord, unintentionally harmonizing your scream with the sound of slippy perishing",
                      "duration_ms": 1000
                    }
                  }
                }
              ]
            }
          }
        ]
      },
      {
        "Action": {
          "Sleep": {
            "tracing_name": "die horribly",
            "duration_ms": 1000
          }
        }
      }
    ]
  },
  "grpc_method_b": null,
  "grpc_method_c": null
}"#;
    let jigsaw = JigsawInstance::new(&json);

    // let addr = "[::1]:50051".parse()?; // apparently ipv6 isn't working, maybe only in docker containers?
    let addr = "127.0.0.1:6379".parse()?;

    Server::builder()
        .add_service(jigsaw.as_server())
        .serve(addr)
        .await?;

    Ok(())
}
