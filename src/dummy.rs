use std::fs::remove_file;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::process;
use std::thread::sleep;
use std::time::Duration;

use erg_common::config::ErgConfig;
use erg_common::error::MultiErrorDisplay;
use erg_common::python_util::spawn_py;
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
    /// Send from client to server. Let the server to execute the code.
    Execute = 0x06,
    /// Informs that it is not a supported instruction.
    Unknown = 0x00,
}

impl From<u8> for Inst {
    fn from(v: u8) -> Inst {
        match v {
            0x01 => Inst::Print,
            0x02 => Inst::Load,
            0x03 => Inst::Exception,
            0x04 => Inst::Initialize,
            0x05 => Inst::Exit,
            0x06 => Inst::Execute,
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
    size: u16,
    data: Option<Vec<u8>>,
}

impl Message {
    fn new(inst: Inst, data: Option<Vec<u8>>) -> Self {
        let size = if let Some(d) = &data {
            if d.len() > usize::from(u16::MAX) {
                eprintln!("Warning: length truncated to 65535");
                u16::MAX
            } else {
                d.len() as u16
            }
        } else {
            0
        };
        Self { inst, size, data }
    }

    #[allow(unused)]
    fn len(&self) -> usize {
        self.size as usize
    }
}

#[derive(Debug)]
struct MessageStream<T: Read + Write> {
    stream: T,
}

impl<T: Read + Write> MessageStream<T> {
    fn new(stream: T) -> Self {
        Self { stream }
    }

    fn send_msg(&mut self, msg: &Message) -> Result<(), std::io::Error> {
        let mut write_buf = Vec::with_capacity(1024);
        write_buf.extend((msg.inst as u8).to_be_bytes());
        write_buf.extend((msg.size).to_be_bytes());
        write_buf.extend_from_slice(&msg.data.clone().unwrap_or_default());

        self.stream.write_all(&write_buf)?;

        Ok(())
    }

    fn recv_msg(&mut self) -> Result<Message, std::io::Error> {
        // read instruction, 1 byte
        let mut inst_buf = [0; 1];
        self.stream.read_exact(&mut inst_buf)?;

        let inst: Inst = u8::from_be_bytes(inst_buf).into();

        // read size, 2 bytes
        let mut size_buf = [0; 2];
        self.stream.read_exact(&mut size_buf)?;

        let data_size = u16::from_be_bytes(size_buf) as usize;

        if data_size == 0 {
            return Ok(Message::new(inst, None));
        }

        // read data
        let mut data_buf = vec![0; data_size];
        self.stream.read_exact(&mut data_buf)?;

        Ok(Message::new(inst, Some(data_buf)))
    }
}

#[test]
fn test_message() {
    use std::collections::VecDeque;

    let inner = Box::<VecDeque<u8>>::default();
    let mut stream = MessageStream::new(inner);

    // test send_msg with data
    stream
        .send_msg(&Message::new(
            Inst::Print,
            Some("hello".chars().map(|c| c as u8).collect()),
        ))
        .unwrap();
    assert_eq!(
        stream.stream.as_slices(),
        (&[1, 0, 5, 104, 101, 108, 108, 111][..], &[][..])
    );

    // test recv_msg
    // data field, 'A' => hex is 0x41
    stream.stream.push_front(0x41);
    // size field
    stream.stream.push_front(0x01);
    stream.stream.push_front(0x00);
    // inst field
    stream.stream.push_front(0x01);

    let msg = stream.recv_msg().unwrap();
    assert_eq!(msg.inst, Inst::Print);
    assert_eq!(msg.len(), 1);
    assert_eq!(std::str::from_utf8(&msg.data.unwrap()).unwrap(), "A");
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
        let src = self.cfg_mut().input.read();
        let art = self.compiler.compile(src, "exec").map_err(|eart| {
            eart.warns.write_all_to(&mut self.cfg_mut().output);
            eart.errors
        })?;
        art.warns.write_all_to(&mut self.cfg_mut().output);
        let stat = art.object.exec(self.cfg()).expect("failed to execute");
        let stat = ExitStatus::new(stat.code().unwrap_or(0), art.warns.len(), 0);
        Ok(stat)
    }

    fn eval(&mut self, src: String) -> Result<String, EvalErrors> {
        let arti = self
            .compiler
            .eval_compile(src, "eval")
            .map_err(|eart| eart.errors)?;
        let ((code, last), warns) = (arti.object, arti.warns);
        let mut res = warns.to_string();

        macro_rules! err_handle {
            () => {{
                self.finish();
                process::exit(1);
            }};
            ($hint:expr $(,$args:expr),* $(,)?) => {{
                self.finish();
                eprintln!($hint, $($args)*);
                process::exit(1);
            }};
        }

        // Tell the REPL server to execute the code
        if let Err(err) = self.stream.as_mut().unwrap().send_msg(&Message::new(
            Inst::Execute,
            Some(
                code.into_script(self.compiler.cfg.py_magic_num)
                    .into_bytes(),
            ),
        )) {
            err_handle!("Sending error: {err}");
        };

        // receive data from server
        let data = match self.stream.as_mut().unwrap().recv_msg() {
            Result::Ok(msg) => {
                let s = match msg.inst {
                    Inst::Exception => {
                        debug_assert!(
                            std::str::from_utf8(msg.data.as_ref().unwrap()) == Ok("SystemExit")
                        );
                        return Err(EvalErrors::from(EvalError::system_exit()));
                    }
                    Inst::Initialize => {
                        self.compiler.initialize_generator();
                        String::from_utf8(msg.data.unwrap_or_default())
                    }
                    Inst::Print => String::from_utf8(msg.data.unwrap_or_default()),
                    Inst::Exit => err_handle!("Receiving inst {:?} from server", msg.inst),
                    // `load` can only be sent from the client to the server
                    Inst::Load | Inst::Execute | Inst::Unknown => {
                        err_handle!("Receiving unexpected inst {:?} from server", msg.inst)
                    }
                };

                if let Ok(ss) = s {
                    ss
                } else {
                    err_handle!("Failed to parse server response data, error: {:?}", s.err());
                }
            }
            Result::Err(err) => err_handle!("Received an error: {err}"),
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
