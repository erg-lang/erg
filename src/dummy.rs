use std::fs::remove_file;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::process;
use std::thread::sleep;
use std::time::Duration;

use erg_common::config::ErgConfig;
use erg_common::error::MultiErrorDisplay;
use erg_common::python_util::{exec_pyc, spawn_py};
use erg_common::traits::{ExitStatus, Runnable, Stream};

use erg_compiler::hir::Expr;
use erg_compiler::ty::HasType;

use erg_compiler::error::{CompileError, CompileErrors};
use erg_compiler::Compiler;

pub type EvalError = CompileError;
pub type EvalErrors = CompileErrors;

/// The instructions for communication between the client and the server.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
enum Inst {
    /// Send from server to client. Informs the client to print data.
    Print = 0x01,
    /// Send from client to server. Informs the REPL server that the executable .pyc file has been written out and is ready for evaluation.
    Load = 0x02,
    /// Send from server to client. Represents an exception.
    Exception = 0x03,
    /// Send from server to client. Tells the code generator to initialize due to an error.
    Initialize = 0x04,
    /// Informs that the connection is to be / should be terminated.
    Exit = 0x05,
    /// Informs that it is not a supported instruction.
    Unknown = 0x00,
}

impl Inst {
    fn has_data(&self) -> bool {
        match self {
            Self::Print => true,
            Self::Load => false,
            Self::Exception => true,
            Self::Initialize => true,
            Self::Exit => false,
            Self::Unknown => false,
        }
    }
}

impl Into<Inst> for u8 {
    fn into(self) -> Inst {
        match self {
            0x01 => Inst::Print,
            0x02 => Inst::Load,
            0x03 => Inst::Exception,
            0x04 => Inst::Initialize,
            0x05 => Inst::Exit,
            _ => Inst::Unknown,
        }
    }
}

/// -------------------------------
/// | ins    | size    | data
/// -------------------------------
/// | 1 byte | 2 bytes | n bytes
/// -------------------------------
#[derive(Debug, Clone)]
struct Message {
    inst: Inst,
    size: usize,
    data: Option<Vec<u8>>,
}

impl Message {
    fn new(inst: Inst, data: Option<Vec<u8>>) -> Self {
        let size = if let Some(d) = &data { d.len() } else { 0 };
        Self { inst, size, data }
    }
}

#[derive(Debug)]
struct MessageStream<T: Read + Write> {
    stream: T,
    read_buf: Vec<u8>,
    write_buf: Vec<u8>,
}

impl<T: Read + Write> MessageStream<T> {
    fn new(stream: T) -> Self {
        Self {
            stream,
            read_buf: Vec::new(),
            write_buf: Vec::new(),
        }
    }

    fn send_msg(&mut self, msg: &Message) -> Result<(), std::io::Error> {
        self.write_buf.clear();

        self.write_buf.extend((msg.inst as u8).to_be_bytes());
        self.write_buf.extend((msg.size).to_be_bytes());
        self.write_buf
            .extend_from_slice(&msg.data.clone().unwrap_or_default());

        self.stream.write_all(&self.write_buf)?;

        Ok(())
    }

    fn recv_msg(&mut self) -> Result<Message, std::io::Error> {
        // read instruction, 1 byte
        let mut inst_buf = [0; 1];
        self.stream.read_exact(&mut inst_buf)?;

        let inst: Inst = u8::from_be_bytes(inst_buf).into();

        if !inst.has_data() {
            return Ok(Message::new(inst, None));
        }

        // read size, 2 bytes
        let mut size_buf = [0; 2];
        self.stream.read_exact(&mut size_buf)?;

        let data_size = u16::from_be_bytes(size_buf) as usize;

        // read data
        let mut data_buf = vec![0; data_size];
        self.stream.read_exact(&mut data_buf)?;

        Ok(Message::new(inst, Some(data_buf)))
    }
}

fn find_available_port() -> u16 {
    let socket = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0);
    TcpListener::bind(socket)
        .and_then(|listener| listener.local_addr())
        .map(|sock_addr| sock_addr.port())
        .expect("No free port found.")
}

/// Open the Python interpreter as a server and act as an Erg interpreter by mediating communication
///
/// Pythonインタープリタをサーバーとして開き、通信を仲介することでErgインタープリタとして振る舞う
#[derive(Debug)]
pub struct DummyVM {
    compiler: Compiler,
    stream: Option<MessageStream<TcpStream>>,
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
                .replace("__PORT__", port.to_string().as_str())
                .replace("__MODULE__", &cfg.dump_filename().replace('/', "."));
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
                        break Some(MessageStream::new(stream));
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
            // send exit to server
            if let Err(err) = stream.send_msg(&Message::new(Inst::Exit, None)) {
                eprintln!("Write error: {err}");
                process::exit(1);
            }

            // wait server exit
            match stream.recv_msg() {
                Result::Ok(msg) => {
                    if msg.inst == Inst::Exit && !self.cfg().quiet_repl {
                        println!("The REPL server is closed.");
                    }
                }
                Result::Err(err) => {
                    eprintln!("Read error: {err}");
                    process::exit(1);
                }
            }

            remove_file(self.cfg().dump_pyc_filename()).unwrap_or(());
        }
    }

    fn initialize(&mut self) {
        self.compiler.initialize();
    }

    fn clear(&mut self) {
        self.compiler.clear();
    }

    fn exec(&mut self) -> Result<ExitStatus, Self::Errs> {
        // Parallel execution is not possible without dumping with a unique file name.
        let filename = self.cfg().dump_pyc_filename();
        let src = self.cfg_mut().input.read();
        let warns = self
            .compiler
            .compile_and_dump_as_pyc(&filename, src, "exec")
            .map_err(|eart| {
                eart.warns.fmt_all_stderr();
                eart.errors
            })?;
        warns.fmt_all_stderr();
        let code = exec_pyc(&filename, self.cfg().py_command, &self.cfg().runtime_args);
        remove_file(&filename).unwrap();
        Ok(ExitStatus::new(code.unwrap_or(1), warns.len(), 0))
    }

    fn eval(&mut self, src: String) -> Result<String, EvalErrors> {
        let path = self.cfg().dump_pyc_filename();
        let arti = self
            .compiler
            .eval_compile_and_dump_as_pyc(path, src, "eval")
            .map_err(|eart| eart.errors)?;
        let (last, warns) = (arti.object, arti.warns);
        let mut res = warns.to_string();

        macro_rules! err_handle {
            () => {
                {
                    self.finish();
                    process::exit(1);

                }
            };
            ($hint:expr, $($args:expr),*) => {
                {
                    self.finish();
                    eprintln!($hint, $($args)*);
                    process::exit(1);
                }
            };
        }

        // Tell the REPL server to execute the code
        if let Err(err) = self
            .stream
            .as_mut()
            .unwrap()
            .send_msg(&Message::new(Inst::Load, None))
        {
            err_handle!("Sending error: {}", err);
        };

        // receive data from server
        let data = match self.stream.as_mut().unwrap().recv_msg() {
            Result::Ok(msg) => {
                let s = match msg.inst {
                    Inst::Exception => {
                        return Err(EvalErrors::from(EvalError::system_exit()));
                    }
                    Inst::Initialize => {
                        self.compiler.initialize_generator();
                        String::from_utf8(msg.data.unwrap_or_default())
                    }
                    Inst::Print => String::from_utf8(msg.data.unwrap_or_default()),
                    Inst::Exit => err_handle!("Recving inst {:?} from server", msg.inst),
                    // `load` can only be sent from the client to the server
                    Inst::Load | Inst::Unknown => {
                        err_handle!("Recving unexpected inst {:?} from server", msg.inst)
                    }
                };

                if s.is_err() {
                    err_handle!("Failed to parse server response data, error: {:?}", s.err());
                } else {
                    s.unwrap()
                }
            }
            Result::Err(err) => err_handle!("Recving error: {}", err),
        };

        res.push_str(&data);
        // If the result of an expression is None, it will not be displayed in the REPL.
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
    pub fn exec(&mut self) -> Result<ExitStatus, EvalErrors> {
        Runnable::exec(self)
    }

    /// Evaluates code passed as a string.
    pub fn eval(&mut self, src: String) -> Result<String, EvalErrors> {
        Runnable::eval(self, src)
    }
}
