# `molc`

`molc` is a mock (fake) language client for testing language servers.

## Usage

You can see specific examples of molc use in [ELS](https://github.com/erg-lang/erg/tree/main/crates/els).

```rust
use lsp_types::{Url, Value};

use molc::{FakeClient, LangServer, RedirectableStdout};
use molc::oneline_range;

pub struct Server {
    stdout_redirect: Option<std::sync::mpsc::Sender<Value>>,
    ...
}

impl LangServer for Server {
    fn dispatch(&mut self, msg: impl Into<Value>) -> Result<(), Box<dyn std::error::Error>> {
        self.dispatch(msg)
    }
}

impl RedirectableStdout for Server {
    fn sender(&self) -> Option<&std::sync::mpsc::Sender<Value>> {
        self.stdout_redirect.as_ref()
    }
}

impl Server {
    fn bind_fake_client() -> FakeClient<Self> {
        // The server should send responses to this channel at least during testing.
        let (sender, receiver) = std::sync::mpsc::channel();
        DummyClient::new(
            Server::new(Some(sender)),
            receiver,
        )
    }

    fn init(&mut self, msg: &Value, id: i64) -> ELSResult<()> {
        self.send_log("initializing the language server")?;
        let result = InitializeResult {
            ...
        };
        self.init_services();
        self.send_stdout(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }))
    }

    ...
}

#[test]
fn test_references() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    let uri = Url::from_file_path(Path::new(FILE_A).canonicalize()?).unwrap();
    client.notify_open(FILE_A)?;
    let locations = client.request_references(uri, 1, 4)?.unwrap();
    assert_eq!(locations.len(), 1);
    assert_eq!(&locations[0].range, &oneline_range(1, 4, 5));
    Ok(())
}
```
