use std::fs::remove_file;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::process;
use std::thread::sleep;
use std::time::Duration;

use erg_common::config::ErgConfig;
use erg_common::error::MultiErrorDisplay;
use erg_common::python_util::{exec_pyc, spawn_py};
use erg_common::traits::Runnable;

use erg_compiler::hir::Expr;
use erg_compiler::ty::HasType;

use erg_compiler::error::{CompileError, CompileErrors};
use erg_compiler::Compiler;

pub type EvalError = CompileError;
pub type EvalErrors = CompileErrors;

/// Open the Python interpreter as a server and act as an Erg interpreter by mediating communication
///
/// Pythonインタープリタをサーバーとして開き、通信を仲介することでErgインタープリタとして振る舞う
#[derive(Debug)]
pub struct DummyVM {
    compiler: Compiler,
    stream: Option<TcpStream>,
}

impl Default for DummyVM {
    fn default() -> Self {
        Self::new(ErgConfig::default())
    }
}

impl Drop for DummyVM {
    fn drop(&mut self) {
        self.finish();
    }
}

impl Runnable for DummyVM {
    type Err = EvalError;
    type Errs = EvalErrors;
    const NAME: &'static str = "Erg interpreter";

    #[inline]
    fn cfg(&self) -> &ErgConfig {
        &self.compiler.cfg
    }
    #[inline]
    fn cfg_mut(&mut self) -> &mut ErgConfig {
        &mut self.compiler.cfg
    }

    fn new(cfg: ErgConfig) -> Self {
        let stream = if cfg.input.is_repl() {
            if !cfg.quiet_repl {
                println!("Starting the REPL server...");
            }
            let port = find_available_port();
            let code = include_str!("scripts/repl_server.py")
                .replace("__PORT__", port.to_string().as_str());
            spawn_py(cfg.py_command, &code);
            let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
            if !cfg.quiet_repl {
                println!("Connecting to the REPL server...");
            }
            loop {
                match TcpStream::connect(addr) {
                    Ok(stream) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(cfg.py_server_timeout)))
                            .unwrap();
                        break Some(stream);
                    }
                    Err(_) => {
                        if !cfg.quiet_repl {
                            println!("Retrying to connect to the REPL server...");
                        }
                        sleep(Duration::from_millis(500));
                        continue;
                    }
                }
            }
        } else {
            None
        };
        Self {
            compiler: Compiler::new(cfg),
            stream,
        }
    }

    fn finish(&mut self) {
        if let Some(stream) = &mut self.stream {
            if let Err(err) = stream.write_all("exit".as_bytes()) {
                eprintln!("Write error: {err}");
                process::exit(1);
            }
            let mut buf = [0; 1024];
            match stream.read(&mut buf) {
                Result::Ok(n) => {
                    let s = std::str::from_utf8(&buf[..n]).unwrap();
                    if s.contains("closed") && !self.cfg().quiet_repl {
                        println!("The REPL server is closed.");
                    }
                }
                Result::Err(err) => {
                    eprintln!("Read error: {err}");
                    process::exit(1);
                }
            }
            remove_file("o.pyc").unwrap_or(());
        }
    }

    fn initialize(&mut self) {
        self.compiler.initialize();
    }

    fn clear(&mut self) {
        self.compiler.clear();
    }

    fn exec(&mut self) -> Result<i32, Self::Errs> {
        // Parallel execution is not possible without dumping with a unique file name.
        let filename = self.cfg().dump_pyc_filename();
        let warns = self
            .compiler
            .compile_and_dump_as_pyc(&filename, self.input().read(), "exec")
            .map_err(|eart| {
                eart.warns.fmt_all_stderr();
                eart.errors
            })?;
        warns.fmt_all_stderr();
        let code = exec_pyc(&filename, self.cfg().py_command, &self.cfg().runtime_args);
        remove_file(&filename).unwrap();
        Ok(code.unwrap_or(1))
    }

    fn eval(&mut self, src: String) -> Result<String, EvalErrors> {
        let arti = self
            .compiler
            .eval_compile_and_dump_as_pyc("o.pyc", src, "eval")
            .map_err(|eart| eart.errors)?;
        let (last, warns) = (arti.object, arti.warns);
        let mut res = warns.to_string();
        // Tell the REPL server to execute the code
        res += &match self.stream.as_mut().unwrap().write("load".as_bytes()) {
            Result::Ok(_) => {
                // read the result from the REPL server
                let mut buf = [0; 1024];
                match self.stream.as_mut().unwrap().read(&mut buf) {
                    Result::Ok(n) => {
                        let s = std::str::from_utf8(&buf[..n])
                            .expect("failed to parse the response, maybe the output is too long");
                        if s == "[Exception] SystemExit" {
                            return Err(EvalErrors::from(EvalError::system_exit()));
                        }
                        s.to_string()
                    }
                    Result::Err(err) => {
                        self.finish();
                        eprintln!("Read error: {err}");
                        process::exit(1);
                    }
                }
            }
            Result::Err(err) => {
                self.finish();
                eprintln!("Sending error: {err}");
                process::exit(1);
            }
        };
        if res.ends_with("None") {
            res.truncate(res.len() - 5);
        }
        if self.cfg().show_type {
            res.push_str(": ");
            res.push_str(
                &last
                    .as_ref()
                    .map(|last| last.t())
                    .unwrap_or_default()
                    .to_string(),
            );
            if let Some(Expr::Def(def)) = last {
                res.push_str(&format!(" ({})", def.sig.ident()));
            }
        }
        Ok(res)
    }
}

impl DummyVM {
    /// Execute the script specified in the configuration.
    pub fn exec(&mut self) -> Result<i32, EvalErrors> {
        Runnable::exec(self)
    }

    /// Evaluates code passed as a string.
    pub fn eval(&mut self, src: String) -> Result<String, EvalErrors> {
        Runnable::eval(self, src)
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
