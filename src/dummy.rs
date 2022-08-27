use std::fs::remove_file;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::thread::sleep;
use std::time::Duration;

use erg_common::config::{ErgConfig, Input};
use erg_common::python_util::{exec_py, exec_pyc};
use erg_common::str::Str;
use erg_common::traits::Runnable;

use erg_compiler::error::{CompileError, CompileErrors};
use erg_compiler::Compiler;

/// Open the Python interpreter as a server and act as an Erg interpreter by mediating communication
///
/// Pythonインタープリタをサーバーとして開き、通信を仲介することでErgインタープリタとして振る舞う
#[derive(Debug)]
pub struct DummyVM {
    cfg: ErgConfig,
    compiler: Compiler,
    stream: Option<TcpStream>,
}

impl Runnable for DummyVM {
    type Err = CompileError;
    type Errs = CompileErrors;
    const NAME: &'static str = "Erg interpreter";

    fn new(cfg: ErgConfig) -> Self {
        let stream = if cfg.input.is_repl() {
            println!("Starting the REPL server...");
            let port = find_available_port();
            let code = include_str!("scripts/repl_server.py")
                .replace("__PORT__", port.to_string().as_str());
            exec_py(&code);
            let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
            println!("Connecting to the REPL server...");
            loop {
                match TcpStream::connect(&addr) {
                    Ok(stream) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(cfg.py_server_timeout)))
                            .unwrap();
                        break Some(stream);
                    }
                    Err(_) => {
                        println!("Retrying to connect to the REPL server...");
                        sleep(Duration::from_millis(500));
                        continue;
                    }
                }
            }
        } else {
            None
        };
        Self {
            compiler: Compiler::new(cfg.copy()),
            cfg,
            stream,
        }
    }

    #[inline]
    fn input(&self) -> &Input {
        &self.cfg.input
    }

    fn finish(&mut self) {
        if let Some(stream) = &mut self.stream {
            stream.write_all("exit".as_bytes()).unwrap();
            let mut buf = [0; 1024];
            match stream.read(&mut buf) {
                Result::Ok(n) => {
                    let s = std::str::from_utf8(&buf[..n]).unwrap();
                    if s.contains("closed") {
                        println!("The REPL server is closed.");
                    }
                }
                Result::Err(e) => {
                    panic!("{}", format!("Read error: {e}"));
                }
            }
        }
    }

    fn clear(&mut self) {
        self.compiler.clear();
    }

    fn exec(&mut self) -> Result<(), Self::Errs> {
        let src = self.input().read();
        self.compiler
            .compile_and_dump_as_pyc(src, "o.pyc", "exec")?;
        exec_pyc("o.pyc");
        remove_file("o.pyc").unwrap();
        Ok(())
    }

    fn eval(&mut self, src: Str) -> Result<String, CompileErrors> {
        self.compiler
            .compile_and_dump_as_pyc(src, "o.pyc", "eval")?;
        let mut res = match self.stream.as_mut().unwrap().write("load".as_bytes()) {
            Result::Ok(_) => {
                let mut buf = [0; 1024];
                match self.stream.as_mut().unwrap().read(&mut buf) {
                    Result::Ok(n) => {
                        let s = std::str::from_utf8(&buf[..n]).unwrap();
                        if s == "[Exception] SystemExit" {
                            return Err(CompileErrors::from(CompileError::system_exit()));
                        }
                        s.to_string()
                    }
                    Result::Err(e) => {
                        self.finish();
                        panic!("{}", format!("Read error: {e}"));
                    }
                }
            }
            Result::Err(e) => {
                self.finish();
                panic!("{}", format!("Sending error: {e}"))
            }
        };
        if res.ends_with("None") {
            res.truncate(res.len() - 5);
        }
        Ok(res)
    }
}

fn find_available_port() -> u16 {
    const DEFAULT_PORT: u16 = 8736;
    TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, DEFAULT_PORT))
        .is_ok()
        .then_some(DEFAULT_PORT)
        .unwrap_or_else(|| {
            let socket = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0);
            TcpListener::bind(socket)
                .and_then(|listener| listener.local_addr())
                .map(|sock_addr| sock_addr.port())
                .expect("No free port found.")
        })
}
